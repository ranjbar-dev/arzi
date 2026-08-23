//! Step 2.5 (docs/phase-2-accounting-core.md §2.5): the generic engine other
//! domains (treasury, inventory) will call to post their own vouchers —
//! built once here, wired to real callers in Phases 4-5. No caller exists
//! yet in this phase; exercised directly by `api/tests/auto_post.rs`.
//!
//! This is where B1/B2 (`specs/00-overview.md` fact 5: the legacy's
//! `MakeSanadU` "has no balance check" at all) get fixed structurally —
//! every input either produces a fully-balanced, fully-written voucher or
//! nothing is written at all. Every line is marked non-manual
//! (`source_module` != 0, matching the caller's `source_kind`), so it's
//! immediately read-only from 2.4's voucher editor.
//!
//! Deliberately takes `&mut Transaction` rather than opening its own (specs/
//! 10-target-architecture.md §2.4) — the caller's source document (an
//! inventory invoice, a cheque, …) and the voucher it generates commit
//! together or not at all, closing the "half-written state" class of defect
//! the target architecture doc calls out.
//!
//! Scope note: idempotent regeneration ("re-posting the same source document
//! replaces its previous lines", specs/03-accounting-core/03-06-b.md §6.6
//! step 1 / §6.8's recommendation) is deliberately NOT built here — the
//! Build bullet only asks for "creates a voucher + lines", and the exact
//! regenerate-vs-reject semantics depend on a real caller's requirements
//! that don't exist until Phase 4/5. Left for whichever of those steps
//! builds the first real caller.

use crate::audit;
use chrono::NaiveDate;
use serde_json::json;
use sqlx::{Postgres, Transaction};

pub struct GeneratedLine {
    pub account_id: i64,
    pub debit: i64,
    pub credit: i64,
    pub description: String,
}

#[derive(Debug)]
pub enum PostingError {
    /// No lines at all — there's nothing to post.
    EmptyLines,
    /// `sum(debit) != sum(credit)`. Checked before any write.
    Unbalanced,
    /// A line's `account_id` doesn't resolve to an account in this tenant.
    AccountNotFound(i64),
    /// A line's account has children — only leaves are postable (03-01-b.md §1.6).
    AccountNotLeaf(i64),
    /// A line has a negative amount, or has both/neither side filled.
    InvalidLineAmount(i64),
    FiscalYearNotFound,
    FiscalYearClosed,
    Database(sqlx::Error),
}

impl From<sqlx::Error> for PostingError {
    fn from(err: sqlx::Error) -> Self {
        PostingError::Database(err)
    }
}

/// Posts `lines` as one new voucher, every line marked generated
/// (`source_module = source_kind`, `source_id`). Validates everything —
/// balance, every account's existence and leaf-ness, the fiscal year being
/// open — before issuing a single `INSERT`, so a bad input on the Nth line
/// leaves nothing written at all (no savepoint/rollback dance needed; the
/// manual test's "force an error partway through" case never gets partway).
pub async fn post_generated_voucher(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: i64,
    fiscal_year_id: i64,
    voucher_date: NaiveDate,
    description: &str,
    source_kind: i16,
    source_id: i64,
    lines: &[GeneratedLine],
    created_by: i64,
) -> Result<i64, PostingError> {
    if lines.is_empty() {
        return Err(PostingError::EmptyLines);
    }

    let total_debit: i64 = lines.iter().map(|l| l.debit).sum();
    let total_credit: i64 = lines.iter().map(|l| l.credit).sum();
    if total_debit != total_credit {
        return Err(PostingError::Unbalanced);
    }

    for line in lines {
        if line.debit < 0 || line.credit < 0 || (line.debit > 0) == (line.credit > 0) {
            return Err(PostingError::InvalidLineAmount(line.account_id));
        }
        let child_count: Option<i32> =
            sqlx::query_scalar("SELECT child_count FROM accounts WHERE tenant_id = $1 AND id = $2")
                .bind(tenant_id)
                .bind(line.account_id)
                .fetch_optional(&mut **tx)
                .await?;
        match child_count {
            None => return Err(PostingError::AccountNotFound(line.account_id)),
            Some(c) if c > 0 => return Err(PostingError::AccountNotLeaf(line.account_id)),
            _ => {}
        }
    }

    let fiscal_year: Option<(bool,)> =
        sqlx::query_as("SELECT is_active FROM fiscal_years WHERE tenant_id = $1 AND id = $2")
            .bind(tenant_id)
            .bind(fiscal_year_id)
            .fetch_optional(&mut **tx)
            .await?;
    match fiscal_year {
        None => return Err(PostingError::FiscalYearNotFound),
        Some((false,)) => return Err(PostingError::FiscalYearClosed),
        _ => {}
    }

    let voucher_number: i32 = sqlx::query_scalar(
        "UPDATE fiscal_years SET next_voucher_number = next_voucher_number + 1 \
         WHERE id = $1 RETURNING next_voucher_number - 1",
    )
    .bind(fiscal_year_id)
    .fetch_one(&mut **tx)
    .await?;

    let voucher_id: i64 = sqlx::query_scalar(
        "INSERT INTO vouchers \
         (tenant_id, fiscal_year_id, voucher_number, voucher_date, description, \
          total_debit, total_credit, line_count, created_by) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9) RETURNING id",
    )
    .bind(tenant_id)
    .bind(fiscal_year_id)
    .bind(voucher_number)
    .bind(voucher_date)
    .bind(description)
    .bind(total_debit)
    .bind(total_credit)
    .bind(lines.len() as i32)
    .bind(created_by)
    .fetch_one(&mut **tx)
    .await?;

    for line in lines {
        sqlx::query(
            "INSERT INTO voucher_lines \
             (tenant_id, voucher_id, fiscal_year_id, line_date, debit_amount, credit_amount, \
              description, account_id, source_module, source_id, created_by) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)",
        )
        .bind(tenant_id)
        .bind(voucher_id)
        .bind(fiscal_year_id)
        .bind(voucher_date)
        .bind(line.debit)
        .bind(line.credit)
        .bind(&line.description)
        .bind(line.account_id)
        .bind(source_kind)
        .bind(source_id)
        .bind(created_by)
        .execute(&mut **tx)
        .await?;
    }

    audit::record_mutation(
        tx,
        tenant_id,
        "vouchers",
        voucher_id,
        "insert",
        Some(created_by),
        None,
        Some(json!({
            "voucherNumber": voucher_number,
            "sourceKind": source_kind,
            "sourceId": source_id,
            "lineCount": lines.len(),
        })),
    )
    .await?;

    Ok(voucher_id)
}
