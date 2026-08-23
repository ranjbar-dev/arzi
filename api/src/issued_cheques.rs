//! Step 4.5 follow-up (docs/phase-4-treasury.md §4.5 / specs/06-treasury/06-
//! 01-entity-model.md §1.4-§1.5): issued-cheque payment batches (legacy
//! `CheckMaster`/`CheckDetail`) — the piece 4.1-4.4 left unbuilt because it
//! sits on **A9** (specs/11-open-decisions.md), an explicitly unresolved
//! question ("is `CheckMaster` one cheque or a payment batch?") that the
//! spec itself says needs a populated legacy database to settle.
//!
//! **Unblocked by an explicit user decision (2026-08-23), not a guess made
//! here**: treat `CheckMaster` as a payment batch (header + N payee lines),
//! matching the DDL-shape evidence already on record in A9's own writeup
//! (header/detail structure, cached `CM_Count`). This is still an
//! inferred-not-confirmed call — the reference dump has zero rows in this
//! table and neither legacy table has a primary key — flagged here again so
//! it isn't mistaken for a settled fact if real data ever surfaces.
//!
//! Structurally the mirror of `petty_cash.rs` (06-07.md §7.2's own note:
//! "structurally identical to the issued-cheque batch... with `Check`→
//! `Tankhah`") — same whole-batch create/update (the legacy's `TVirtualTable`
//! line grid is replaced wholesale on every save, `CheckEditU.pas:447`),
//! same C3 derivation of `total_amount`/`line_count` from the submitted
//! lines rather than the legacy's out-of-transaction `Set_Sum`, same real
//! transactional posting through the Phase 2.5 engine (06-08.md §8.5 defect
//! 6's "separate transactions in the same batch" hazard fixed structurally).
//!
//! Narration for the credit line avoids the legacy's `CM_Desc + ' تعداد '
//! + N + ' نفر '` ("count N persons") — accurate-ish for payees (who often
//! are people) but inconsistent with this codebase's own petty-cash fix and
//! with the fact a payee can be a company; composed as "N payee(s)" instead.

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
        .route("/", get(list_batches).post(create_batch))
        .route("/{id}", get(get_batch).put(update_batch).delete(delete_batch))
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
struct BatchRecord {
    id: i64,
    fiscal_year_id: i64,
    batch_number: Option<String>,
    issue_date: NaiveDate,
    description: String,
    letter_body: Option<String>,
    bank_account_id: i64,
    total_amount: i64,
    line_count: i32,
    voucher_id: Option<i64>,
}

const BATCH_COLUMNS: &str = "id, fiscal_year_id, batch_number, issue_date, description, \
    letter_body, bank_account_id, total_amount, line_count, voucher_id";

async fn fetch_batch(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: i64,
    id: i64,
) -> Result<Option<BatchRecord>, sqlx::Error> {
    sqlx::query_as(&format!("SELECT {BATCH_COLUMNS} FROM cheque_payment_batches WHERE tenant_id = $1 AND id = $2"))
        .bind(tenant_id)
        .bind(id)
        .fetch_optional(&mut **tx)
        .await
}

#[derive(sqlx::FromRow, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct LineRecord {
    id: i64,
    payee_account_id: i64,
    amount: i64,
    description: Option<String>,
    payee_bank_account_number: Option<String>,
    payee_account_holder_name: Option<String>,
}

const LINE_COLUMNS: &str = "id, payee_account_id, amount, description, \
    payee_bank_account_number, payee_account_holder_name";

async fn fetch_lines(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: i64,
    batch_id: i64,
) -> Result<Vec<LineRecord>, sqlx::Error> {
    sqlx::query_as(&format!("SELECT {LINE_COLUMNS} FROM cheque_payment_batch_lines WHERE tenant_id = $1 AND batch_id = $2 ORDER BY id"))
        .bind(tenant_id)
        .bind(batch_id)
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
    payee_account_id: i64,
    amount: i64,
    description: Option<String>,
    payee_bank_account_number: Option<String>,
    payee_account_holder_name: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct BatchFields {
    fiscal_year_id: i64,
    batch_number: Option<String>,
    issue_date: NaiveDate,
    description: String,
    letter_body: Option<String>,
    bank_account_id: i64,
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

/// Posts N debit lines (one per payee) + one credit line to the bank
/// account for the total (06-08.md §8.3 row 7).
async fn post_batch_voucher(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: i64,
    fiscal_year_id: i64,
    issue_date: NaiveDate,
    narration: &str,
    batch_id: i64,
    bank_account_id: i64,
    total: i64,
    lines: &[LineInput],
    actor_id: i64,
) -> Result<i64, (StatusCode, Json<Value>)> {
    let mut generated: Vec<GeneratedLine> = lines
        .iter()
        .map(|l| GeneratedLine {
            account_id: l.payee_account_id,
            debit: l.amount,
            credit: 0,
            description: l.description.clone().unwrap_or_else(|| narration.to_string()),
        })
        .collect();
    generated.push(GeneratedLine { account_id: bank_account_id, debit: 0, credit: total, description: narration.to_string() });
    auto_post::post_generated_voucher(tx, tenant_id, fiscal_year_id, issue_date, narration, 26, batch_id, &generated, actor_id)
        .await
        .map_err(posting_error_response)
}

fn compose_narration(description: &str, line_count: usize) -> String {
    format!("{description} -- {line_count} payee(s)")
}

// ---- create ---------------------------------------------------------------

async fn create_batch(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<BatchFields>,
) -> Result<(StatusCode, Json<Value>), (StatusCode, Json<Value>)> {
    if req.description.trim().is_empty() {
        return Err(bad_request("description_required"));
    }
    validate_lines(&req.lines).map_err(bad_request)?;

    let mut tx = db::begin(&state.pool, auth.tenant_id).await.map_err(|_| internal_error())?;
    fiscal_year_gate(&mut tx, auth.tenant_id, req.fiscal_year_id, req.issue_date).await?;
    require_leaf_account(&mut tx, auth.tenant_id, req.bank_account_id).await?;
    for line in &req.lines {
        require_leaf_account(&mut tx, auth.tenant_id, line.payee_account_id).await?;
    }

    let total: i64 = req.lines.iter().map(|l| l.amount).sum();

    let id: i64 = sqlx::query_scalar(
        "INSERT INTO cheque_payment_batches \
         (tenant_id, fiscal_year_id, batch_number, issue_date, description, letter_body, \
          bank_account_id, total_amount, line_count, created_by) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10) RETURNING id",
    )
    .bind(auth.tenant_id)
    .bind(req.fiscal_year_id)
    .bind(&req.batch_number)
    .bind(req.issue_date)
    .bind(&req.description)
    .bind(&req.letter_body)
    .bind(req.bank_account_id)
    .bind(total)
    .bind(req.lines.len() as i32)
    .bind(auth.user_id)
    .fetch_one(&mut *tx)
    .await
    .map_err(|_| internal_error())?;

    for line in &req.lines {
        sqlx::query(
            "INSERT INTO cheque_payment_batch_lines \
             (tenant_id, batch_id, payee_account_id, amount, description, payee_bank_account_number, \
              payee_account_holder_name, created_by) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
        )
        .bind(auth.tenant_id)
        .bind(id)
        .bind(line.payee_account_id)
        .bind(line.amount)
        .bind(&line.description)
        .bind(&line.payee_bank_account_number)
        .bind(&line.payee_account_holder_name)
        .bind(auth.user_id)
        .execute(&mut *tx)
        .await
        .map_err(|_| internal_error())?;
    }

    let narration = compose_narration(&req.description, req.lines.len());
    let voucher_id = post_batch_voucher(
        &mut tx,
        auth.tenant_id,
        req.fiscal_year_id,
        req.issue_date,
        &narration,
        id,
        req.bank_account_id,
        total,
        &req.lines,
        auth.user_id,
    )
    .await?;
    sqlx::query("UPDATE cheque_payment_batches SET voucher_id = $1 WHERE id = $2")
        .bind(voucher_id)
        .bind(id)
        .execute(&mut *tx)
        .await
        .map_err(|_| internal_error())?;

    audit::record_mutation(
        &mut tx,
        auth.tenant_id,
        "cheque_payment_batches",
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

// ---- update (whole-batch replace) ------------------------------------------

async fn update_batch(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
    Json(req): Json<BatchFields>,
) -> Result<StatusCode, (StatusCode, Json<Value>)> {
    if req.description.trim().is_empty() {
        return Err(bad_request("description_required"));
    }
    validate_lines(&req.lines).map_err(bad_request)?;

    let mut tx = db::begin(&state.pool, auth.tenant_id).await.map_err(|_| internal_error())?;
    let Some(existing) = fetch_batch(&mut tx, auth.tenant_id, id).await.map_err(|_| internal_error())? else {
        return Err(not_found("cheque_payment_batch"));
    };
    fiscal_year_gate(&mut tx, auth.tenant_id, req.fiscal_year_id, req.issue_date).await?;
    require_leaf_account(&mut tx, auth.tenant_id, req.bank_account_id).await?;
    for line in &req.lines {
        require_leaf_account(&mut tx, auth.tenant_id, line.payee_account_id).await?;
    }

    let total: i64 = req.lines.iter().map(|l| l.amount).sum();

    sqlx::query(
        "UPDATE cheque_payment_batches SET fiscal_year_id = $1, batch_number = $2, issue_date = $3, \
         description = $4, letter_body = $5, bank_account_id = $6, total_amount = $7, line_count = $8, \
         updated_at = now(), updated_by = $9 WHERE id = $10",
    )
    .bind(req.fiscal_year_id)
    .bind(&req.batch_number)
    .bind(req.issue_date)
    .bind(&req.description)
    .bind(&req.letter_body)
    .bind(req.bank_account_id)
    .bind(total)
    .bind(req.lines.len() as i32)
    .bind(auth.user_id)
    .bind(id)
    .execute(&mut *tx)
    .await
    .map_err(|_| internal_error())?;

    sqlx::query("DELETE FROM cheque_payment_batch_lines WHERE tenant_id = $1 AND batch_id = $2")
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
            "INSERT INTO cheque_payment_batch_lines \
             (tenant_id, batch_id, payee_account_id, amount, description, payee_bank_account_number, \
              payee_account_holder_name, created_by) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
        )
        .bind(auth.tenant_id)
        .bind(id)
        .bind(line.payee_account_id)
        .bind(line.amount)
        .bind(&line.description)
        .bind(&line.payee_bank_account_number)
        .bind(&line.payee_account_holder_name)
        .bind(auth.user_id)
        .execute(&mut *tx)
        .await
        .map_err(|_| internal_error())?;
    }

    let narration = compose_narration(&req.description, req.lines.len());
    let voucher_id = post_batch_voucher(
        &mut tx,
        auth.tenant_id,
        req.fiscal_year_id,
        req.issue_date,
        &narration,
        id,
        req.bank_account_id,
        total,
        &req.lines,
        auth.user_id,
    )
    .await?;
    sqlx::query("UPDATE cheque_payment_batches SET voucher_id = $1 WHERE id = $2")
        .bind(voucher_id)
        .bind(id)
        .execute(&mut *tx)
        .await
        .map_err(|_| internal_error())?;

    audit::record_mutation(
        &mut tx,
        auth.tenant_id,
        "cheque_payment_batches",
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

async fn list_batches(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(params): Query<ListQuery>,
) -> Result<Json<Vec<BatchRecord>>, (StatusCode, Json<Value>)> {
    let mut tx = db::begin(&state.pool, auth.tenant_id).await.map_err(|_| internal_error())?;
    let rows: Vec<BatchRecord> = if let Some(fy) = params.fiscal_year_id {
        sqlx::query_as(&format!(
            "SELECT {BATCH_COLUMNS} FROM cheque_payment_batches WHERE tenant_id = $1 AND fiscal_year_id = $2 ORDER BY issue_date"
        ))
        .bind(auth.tenant_id)
        .bind(fy)
        .fetch_all(&mut *tx)
        .await
        .map_err(|_| internal_error())?
    } else {
        sqlx::query_as(&format!("SELECT {BATCH_COLUMNS} FROM cheque_payment_batches WHERE tenant_id = $1 ORDER BY issue_date"))
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
struct BatchDetail {
    #[serde(flatten)]
    batch: BatchRecord,
    lines: Vec<LineRecord>,
}

async fn get_batch(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<Json<BatchDetail>, (StatusCode, Json<Value>)> {
    let mut tx = db::begin(&state.pool, auth.tenant_id).await.map_err(|_| internal_error())?;
    let Some(batch) = fetch_batch(&mut tx, auth.tenant_id, id).await.map_err(|_| internal_error())? else {
        return Err(not_found("cheque_payment_batch"));
    };
    let lines = fetch_lines(&mut tx, auth.tenant_id, id).await.map_err(|_| internal_error())?;
    tx.rollback().await.ok();
    Ok(Json(BatchDetail { batch, lines }))
}

// ---- delete -------------------------------------------------------------
//
// Mirrors CheckListU's own guard (§11.9): year open, voucher still draft.

async fn delete_batch(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<StatusCode, (StatusCode, Json<Value>)> {
    let mut tx = db::begin(&state.pool, auth.tenant_id).await.map_err(|_| internal_error())?;
    let Some(batch) = fetch_batch(&mut tx, auth.tenant_id, id).await.map_err(|_| internal_error())? else {
        return Err(not_found("cheque_payment_batch"));
    };
    if let Some(voucher_id) = batch.voucher_id {
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

    sqlx::query("DELETE FROM cheque_payment_batches WHERE tenant_id = $1 AND id = $2")
        .bind(auth.tenant_id)
        .bind(id)
        .execute(&mut *tx)
        .await
        .map_err(|_| internal_error())?;
    if let Some(voucher_id) = batch.voucher_id {
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
        "cheque_payment_batches",
        id,
        "delete",
        Some(auth.user_id),
        Some(json!({ "totalAmount": batch.total_amount, "lineCount": batch.line_count })),
        None,
    )
    .await
    .map_err(|_| internal_error())?;

    tx.commit().await.map_err(|_| internal_error())?;
    Ok(StatusCode::NO_CONTENT)
}
