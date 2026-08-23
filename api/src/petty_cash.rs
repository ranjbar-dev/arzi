//! Step 4.4, petty-cash half (docs/phase-4-treasury.md §4.4 / specs/06-
//! treasury/06-07-petty-cash-tankhah.md): the legacy `TankhahMaster` +
//! `TankhahDetail` — a claim header + N expense lines, no lifecycle, no
//! fund/float/advance/replenishment concept anywhere (§7.1's exhaustive
//! "none of the following exists" list — not built here either, out of
//! scope by the spec's own ruling, not an oversight).
//!
//! **Total is computed from the lines, never entered** (§7.2, C3 "derive,
//! don't store") — maintained in the same transaction as every line write,
//! not the legacy's out-of-transaction `Set_Sum` recompute.
//!
//! **"N persons" narration-bug fix (§7.3):** the legacy's credit-line
//! narration is copy-paste residue from the issued-cheque batch screen
//! ("... count N persons") describing expense lines as people. This module
//! composes a narration that actually describes expense lines.
//!
//! **Real transactional posting (06-08.md §8.5 defect 6):** N debit lines
//! (one per expense line) + one credit line to the custodian, posted
//! through the Phase 2.5 engine inside the same transaction as the claim
//! and its lines — all-or-nothing, same as every other Phase 4 document.
//!
//! Whole-claim create/update (not incremental per-line CRUD like vouchers.rs
//! or accounts.rs) — a judgment call: unlike a voucher, a petty-cash claim
//! has no draft/confirm/post lifecycle to edit incrementally against (§7.1),
//! its lines are always known and submitted together in one form (§7.6's
//! `TankhahEditAddu` is a modal add-one-line dialog, but the header save is
//! still one whole-grid write), and the total's C3 derivation is trivial to
//! keep correct when the whole line set is replaced atomically rather than
//! incrementally maintained across N separate requests.

use crate::{audit, auto_post::{self, GeneratedLine, PostingError}, db, auth::AuthUser, AppState};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    routing::get,
    Json, Router,
};
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::{Postgres, Transaction};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(list_claims).post(create_claim))
        .route("/{id}", get(get_claim).put(update_claim).delete(delete_claim))
}

fn internal_error() -> (StatusCode, Json<Value>) {
    (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": "internal_error" })))
}
fn not_found(what: &str) -> (StatusCode, Json<Value>) {
    (StatusCode::NOT_FOUND, Json(json!({ "error": format!("{what}_not_found") })))
}
fn bad_request(error: &str) -> (StatusCode, Json<Value>) {
    (StatusCode::BAD_REQUEST, Json(json!({ "error": error })))
}

#[derive(sqlx::FromRow, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct ClaimRecord {
    id: i64,
    fiscal_year_id: i64,
    claim_number: Option<String>,
    claim_date: NaiveDate,
    description: Option<String>,
    custodian_account_id: i64,
    total_amount: i64,
    line_count: i32,
    voucher_id: Option<i64>,
}

const CLAIM_COLUMNS: &str = "id, fiscal_year_id, claim_number, claim_date, description, \
    custodian_account_id, total_amount, line_count, voucher_id";

async fn fetch_claim(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: i64,
    id: i64,
) -> Result<Option<ClaimRecord>, sqlx::Error> {
    sqlx::query_as(&format!("SELECT {CLAIM_COLUMNS} FROM petty_cash_claims WHERE tenant_id = $1 AND id = $2"))
        .bind(tenant_id)
        .bind(id)
        .fetch_optional(&mut **tx)
        .await
}

#[derive(sqlx::FromRow, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct LineRecord {
    id: i64,
    expense_account_id: i64,
    amount: i64,
    description: Option<String>,
}

const LINE_COLUMNS: &str = "id, expense_account_id, amount, description";

async fn fetch_lines(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: i64,
    claim_id: i64,
) -> Result<Vec<LineRecord>, sqlx::Error> {
    sqlx::query_as(&format!("SELECT {LINE_COLUMNS} FROM petty_cash_claim_lines WHERE tenant_id = $1 AND claim_id = $2 ORDER BY id"))
        .bind(tenant_id)
        .bind(claim_id)
        .fetch_all(&mut **tx)
        .await
}

async fn require_leaf_account(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: i64,
    account_id: i64,
) -> Result<(), (StatusCode, Json<Value>)> {
    let child_count: Option<i32> =
        sqlx::query_scalar("SELECT child_count FROM accounts WHERE tenant_id = $1 AND id = $2")
            .bind(tenant_id)
            .bind(account_id)
            .fetch_optional(&mut **tx)
            .await
            .map_err(|_| internal_error())?;
    match child_count {
        None => Err(not_found("account")),
        Some(c) if c > 0 => Err(bad_request("account_not_leaf")),
        _ => Ok(()),
    }
}

async fn fiscal_year_gate(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: i64,
    fiscal_year_id: i64,
    date: NaiveDate,
) -> Result<(), (StatusCode, Json<Value>)> {
    let year: Option<(bool, NaiveDate, NaiveDate)> = sqlx::query_as(
        "SELECT is_active, start_date, end_date FROM fiscal_years WHERE tenant_id = $1 AND id = $2",
    )
    .bind(tenant_id)
    .bind(fiscal_year_id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(|_| internal_error())?;
    let Some((is_active, start, end)) = year else {
        return Err(not_found("fiscal_year"));
    };
    if !is_active {
        return Err(bad_request("fiscal_year_closed"));
    }
    if date < start || date > end {
        return Err(bad_request("date_outside_fiscal_year"));
    }
    Ok(())
}

fn posting_error_response(err: PostingError) -> (StatusCode, Json<Value>) {
    match err {
        PostingError::AccountNotFound(_) => not_found("account"),
        PostingError::AccountNotLeaf(_) => bad_request("account_not_leaf"),
        PostingError::FiscalYearNotFound => not_found("fiscal_year"),
        PostingError::FiscalYearClosed => bad_request("fiscal_year_closed"),
        PostingError::EmptyLines => bad_request("at_least_one_line_required"),
        PostingError::Unbalanced | PostingError::InvalidLineAmount(_) | PostingError::Database(_) => internal_error(),
    }
}

// ---- request shape -----------------------------------------------------

#[derive(Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct LineInput {
    expense_account_id: i64,
    amount: i64,
    description: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ClaimFields {
    fiscal_year_id: i64,
    claim_number: Option<String>,
    claim_date: NaiveDate,
    description: Option<String>,
    custodian_account_id: i64,
    lines: Vec<LineInput>,
}

fn validate_lines(lines: &[LineInput]) -> Result<(), &'static str> {
    if lines.is_empty() {
        return Err("at_least_one_line_required");
    }
    if lines.iter().any(|l| l.amount <= 0) {
        return Err("line_amount_must_be_positive");
    }
    Ok(())
}

/// Posts N debit lines (one per expense line) + one credit line to the
/// custodian for the total (§7.3). Narration describes expense lines, never
/// "N persons" (the copy-paste bug this fixes).
async fn post_claim_voucher(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: i64,
    fiscal_year_id: i64,
    claim_date: NaiveDate,
    narration: &str,
    claim_id: i64,
    custodian_account_id: i64,
    total: i64,
    lines: &[LineInput],
    actor_id: i64,
) -> Result<i64, (StatusCode, Json<Value>)> {
    let mut generated: Vec<GeneratedLine> = lines
        .iter()
        .map(|l| GeneratedLine {
            account_id: l.expense_account_id,
            debit: l.amount,
            credit: 0,
            description: l.description.clone().unwrap_or_else(|| narration.to_string()),
        })
        .collect();
    generated.push(GeneratedLine {
        account_id: custodian_account_id,
        debit: 0,
        credit: total,
        description: narration.to_string(),
    });
    auto_post::post_generated_voucher(tx, tenant_id, fiscal_year_id, claim_date, narration, 41, claim_id, &generated, actor_id)
        .await
        .map_err(posting_error_response)
}

fn compose_narration(description: &Option<String>, line_count: usize) -> String {
    description.clone().unwrap_or_else(|| format!("Petty cash claim -- {line_count} expense line(s)"))
}

// ---- create ---------------------------------------------------------------

async fn create_claim(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<ClaimFields>,
) -> Result<(StatusCode, Json<Value>), (StatusCode, Json<Value>)> {
    validate_lines(&req.lines).map_err(bad_request)?;

    let mut tx = db::begin(&state.pool, auth.tenant_id).await.map_err(|_| internal_error())?;
    fiscal_year_gate(&mut tx, auth.tenant_id, req.fiscal_year_id, req.claim_date).await?;
    require_leaf_account(&mut tx, auth.tenant_id, req.custodian_account_id).await?;
    for line in &req.lines {
        require_leaf_account(&mut tx, auth.tenant_id, line.expense_account_id).await?;
    }

    let total: i64 = req.lines.iter().map(|l| l.amount).sum();

    let id: i64 = sqlx::query_scalar(
        "INSERT INTO petty_cash_claims \
         (tenant_id, fiscal_year_id, claim_number, claim_date, description, custodian_account_id, \
          total_amount, line_count, created_by) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9) RETURNING id",
    )
    .bind(auth.tenant_id)
    .bind(req.fiscal_year_id)
    .bind(&req.claim_number)
    .bind(req.claim_date)
    .bind(&req.description)
    .bind(req.custodian_account_id)
    .bind(total)
    .bind(req.lines.len() as i32)
    .bind(auth.user_id)
    .fetch_one(&mut *tx)
    .await
    .map_err(|_| internal_error())?;

    for line in &req.lines {
        sqlx::query(
            "INSERT INTO petty_cash_claim_lines (tenant_id, claim_id, expense_account_id, amount, description, created_by) \
             VALUES ($1, $2, $3, $4, $5, $6)",
        )
        .bind(auth.tenant_id)
        .bind(id)
        .bind(line.expense_account_id)
        .bind(line.amount)
        .bind(&line.description)
        .bind(auth.user_id)
        .execute(&mut *tx)
        .await
        .map_err(|_| internal_error())?;
    }

    let narration = compose_narration(&req.description, req.lines.len());
    let voucher_id = post_claim_voucher(
        &mut tx,
        auth.tenant_id,
        req.fiscal_year_id,
        req.claim_date,
        &narration,
        id,
        req.custodian_account_id,
        total,
        &req.lines,
        auth.user_id,
    )
    .await?;
    sqlx::query("UPDATE petty_cash_claims SET voucher_id = $1 WHERE id = $2")
        .bind(voucher_id)
        .bind(id)
        .execute(&mut *tx)
        .await
        .map_err(|_| internal_error())?;

    audit::record_mutation(
        &mut tx,
        auth.tenant_id,
        "petty_cash_claims",
        id,
        "insert",
        Some(auth.user_id),
        None,
        Some(json!({ "totalAmount": total, "lineCount": req.lines.len() })),
    )
    .await
    .map_err(|_| internal_error())?;

    tx.commit().await.map_err(|_| internal_error())?;
    Ok((StatusCode::CREATED, Json(json!({ "id": id }))))
}

// ---- update (whole-claim replace) ------------------------------------------

async fn update_claim(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
    Json(req): Json<ClaimFields>,
) -> Result<StatusCode, (StatusCode, Json<Value>)> {
    validate_lines(&req.lines).map_err(bad_request)?;

    let mut tx = db::begin(&state.pool, auth.tenant_id).await.map_err(|_| internal_error())?;
    let Some(existing) = fetch_claim(&mut tx, auth.tenant_id, id).await.map_err(|_| internal_error())? else {
        return Err(not_found("petty_cash_claim"));
    };
    fiscal_year_gate(&mut tx, auth.tenant_id, req.fiscal_year_id, req.claim_date).await?;
    require_leaf_account(&mut tx, auth.tenant_id, req.custodian_account_id).await?;
    for line in &req.lines {
        require_leaf_account(&mut tx, auth.tenant_id, line.expense_account_id).await?;
    }

    let total: i64 = req.lines.iter().map(|l| l.amount).sum();

    sqlx::query(
        "UPDATE petty_cash_claims SET fiscal_year_id = $1, claim_number = $2, claim_date = $3, \
         description = $4, custodian_account_id = $5, total_amount = $6, line_count = $7, \
         updated_at = now(), updated_by = $8 WHERE id = $9",
    )
    .bind(req.fiscal_year_id)
    .bind(&req.claim_number)
    .bind(req.claim_date)
    .bind(&req.description)
    .bind(req.custodian_account_id)
    .bind(total)
    .bind(req.lines.len() as i32)
    .bind(auth.user_id)
    .bind(id)
    .execute(&mut *tx)
    .await
    .map_err(|_| internal_error())?;

    // Whole-line replace: delete the old lines and old voucher, reinsert.
    sqlx::query("DELETE FROM petty_cash_claim_lines WHERE tenant_id = $1 AND claim_id = $2")
        .bind(auth.tenant_id)
        .bind(id)
        .execute(&mut *tx)
        .await
        .map_err(|_| internal_error())?;
    if let Some(old_voucher_id) = existing.voucher_id {
        sqlx::query("DELETE FROM vouchers WHERE tenant_id = $1 AND id = $2")
            .bind(auth.tenant_id)
            .bind(old_voucher_id)
            .execute(&mut *tx)
            .await
            .map_err(|_| internal_error())?;
    }

    for line in &req.lines {
        sqlx::query(
            "INSERT INTO petty_cash_claim_lines (tenant_id, claim_id, expense_account_id, amount, description, created_by) \
             VALUES ($1, $2, $3, $4, $5, $6)",
        )
        .bind(auth.tenant_id)
        .bind(id)
        .bind(line.expense_account_id)
        .bind(line.amount)
        .bind(&line.description)
        .bind(auth.user_id)
        .execute(&mut *tx)
        .await
        .map_err(|_| internal_error())?;
    }

    let narration = compose_narration(&req.description, req.lines.len());
    let voucher_id = post_claim_voucher(
        &mut tx,
        auth.tenant_id,
        req.fiscal_year_id,
        req.claim_date,
        &narration,
        id,
        req.custodian_account_id,
        total,
        &req.lines,
        auth.user_id,
    )
    .await?;
    sqlx::query("UPDATE petty_cash_claims SET voucher_id = $1 WHERE id = $2")
        .bind(voucher_id)
        .bind(id)
        .execute(&mut *tx)
        .await
        .map_err(|_| internal_error())?;

    audit::record_mutation(
        &mut tx,
        auth.tenant_id,
        "petty_cash_claims",
        id,
        "update",
        Some(auth.user_id),
        Some(json!({ "totalAmount": existing.total_amount })),
        Some(json!({ "totalAmount": total })),
    )
    .await
    .map_err(|_| internal_error())?;

    tx.commit().await.map_err(|_| internal_error())?;
    Ok(StatusCode::NO_CONTENT)
}

// ---- list / get -------------------------------------------------------------

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ListQuery {
    fiscal_year_id: Option<i64>,
}

async fn list_claims(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(params): Query<ListQuery>,
) -> Result<Json<Vec<ClaimRecord>>, (StatusCode, Json<Value>)> {
    let mut tx = db::begin(&state.pool, auth.tenant_id).await.map_err(|_| internal_error())?;
    let rows: Vec<ClaimRecord> = if let Some(fy) = params.fiscal_year_id {
        sqlx::query_as(&format!(
            "SELECT {CLAIM_COLUMNS} FROM petty_cash_claims WHERE tenant_id = $1 AND fiscal_year_id = $2 ORDER BY claim_date"
        ))
        .bind(auth.tenant_id)
        .bind(fy)
        .fetch_all(&mut *tx)
        .await
        .map_err(|_| internal_error())?
    } else {
        sqlx::query_as(&format!("SELECT {CLAIM_COLUMNS} FROM petty_cash_claims WHERE tenant_id = $1 ORDER BY claim_date"))
            .bind(auth.tenant_id)
            .fetch_all(&mut *tx)
            .await
            .map_err(|_| internal_error())?
    };
    tx.rollback().await.ok();
    Ok(Json(rows))
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ClaimDetail {
    #[serde(flatten)]
    claim: ClaimRecord,
    lines: Vec<LineRecord>,
}

async fn get_claim(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<Json<ClaimDetail>, (StatusCode, Json<Value>)> {
    let mut tx = db::begin(&state.pool, auth.tenant_id).await.map_err(|_| internal_error())?;
    let Some(claim) = fetch_claim(&mut tx, auth.tenant_id, id).await.map_err(|_| internal_error())? else {
        return Err(not_found("petty_cash_claim"));
    };
    let lines = fetch_lines(&mut tx, auth.tenant_id, id).await.map_err(|_| internal_error())?;
    tx.rollback().await.ok();
    Ok(Json(ClaimDetail { claim, lines }))
}

// ---- delete -------------------------------------------------------------

async fn delete_claim(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<StatusCode, (StatusCode, Json<Value>)> {
    let mut tx = db::begin(&state.pool, auth.tenant_id).await.map_err(|_| internal_error())?;
    let Some(claim) = fetch_claim(&mut tx, auth.tenant_id, id).await.map_err(|_| internal_error())? else {
        return Err(not_found("petty_cash_claim"));
    };
    if let Some(voucher_id) = claim.voucher_id {
        let status: Option<String> = sqlx::query_scalar("SELECT status::text FROM vouchers WHERE tenant_id = $1 AND id = $2")
            .bind(auth.tenant_id)
            .bind(voucher_id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(|_| internal_error())?;
        if status.as_deref() != Some("draft") {
            return Err(bad_request("voucher_not_draft"));
        }
    }

    // Cascades the claim's lines automatically (ON DELETE CASCADE).
    sqlx::query("DELETE FROM petty_cash_claims WHERE tenant_id = $1 AND id = $2")
        .bind(auth.tenant_id)
        .bind(id)
        .execute(&mut *tx)
        .await
        .map_err(|_| internal_error())?;
    if let Some(voucher_id) = claim.voucher_id {
        sqlx::query("DELETE FROM vouchers WHERE tenant_id = $1 AND id = $2")
            .bind(auth.tenant_id)
            .bind(voucher_id)
            .execute(&mut *tx)
            .await
            .map_err(|_| internal_error())?;
    }

    audit::record_mutation(
        &mut tx,
        auth.tenant_id,
        "petty_cash_claims",
        id,
        "delete",
        Some(auth.user_id),
        Some(json!({ "totalAmount": claim.total_amount, "lineCount": claim.line_count })),
        None,
    )
    .await
    .map_err(|_| internal_error())?;

    tx.commit().await.map_err(|_| internal_error())?;
    Ok(StatusCode::NO_CONTENT)
}
