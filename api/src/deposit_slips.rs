//! Step 4.4, deposit-slip half (docs/phase-4-treasury.md §4.4 / specs/06-
//! treasury/06-06-deposit-slips-fish.md): the legacy `DFish` — a flat,
//! lifecycle-free money-in event, never a batching document (§6.1: "a Fish
//! groups nothing" — no line items, one amount, one counterparty, one bank
//! account, ported verbatim as a single-row document).
//!
//! **Narration-swap fix (06-08.md §8.5 defect 2):** the legacy builds two
//! separate strings and attaches them to the wrong lines ("by <payer>" lands
//! on the bank-debit line, "to <bank>" on the payer-credit line). This
//! module composes ONE narration and applies it to both voucher lines via
//! `post_transition_voucher`-style posting, so there are no two strings to
//! ever get swapped in the first place — the fix is structural, not a
//! corrected pair of format strings.
//!
//! **Real transactional posting (06-08.md §8.5 defect 6):** posts through
//! the Phase 2.5 engine inside the same transaction as the row insert, same
//! as 4.2's cheques — no separate-transactions-in-one-batch hazard.
//!
//! Channel (`deposit_channel`) is purely descriptive (§6.2) — affects only
//! the narration text, no account, no validation, no posting logic.
//!
//! Delete guard mirrors the legacy's own (`FishListD.pas:257-290`): the
//! linked voucher must still be `draft` (a confirmed/posted voucher means
//! someone downstream is relying on this posting; deleting out from under
//! it would be a worse hazard than the legacy's blanket refusal), and
//! `source_module != 0` (a slip created by another domain, e.g. a future
//! invoice-settlement caller) can never be deleted here — "use the side
//! program" (§6.5), though no such caller exists yet in this phase.

use crate::{
    audit,
    auth::AuthUser,
    auto_post::{self, GeneratedLine, PostingError},
    db, AppState,
};
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
        .route("/", get(list_slips).post(create_slip))
        .route("/{id}", get(get_slip).put(update_slip).delete(delete_slip))
}

fn internal_error() -> (StatusCode, Json<Value>) {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(json!({ "error": "internal_error" })),
    )
}
fn not_found(what: &str) -> (StatusCode, Json<Value>) {
    (
        StatusCode::NOT_FOUND,
        Json(json!({ "error": format!("{what}_not_found") })),
    )
}
fn bad_request(error: &str) -> (StatusCode, Json<Value>) {
    (StatusCode::BAD_REQUEST, Json(json!({ "error": error })))
}

#[derive(sqlx::FromRow, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct SlipRecord {
    id: i64,
    fiscal_year_id: i64,
    slip_number: Option<String>,
    slip_date: NaiveDate,
    amount: i64,
    description: Option<String>,
    payer_account_id: i64,
    bank_account_id: i64,
    channel: String,
    voucher_id: Option<i64>,
    source_module: i16,
}

const SLIP_COLUMNS: &str = "id, fiscal_year_id, slip_number, slip_date, amount, description, \
    payer_account_id, bank_account_id, channel::text, voucher_id, source_module";

async fn fetch_slip(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: i64,
    id: i64,
) -> Result<Option<SlipRecord>, sqlx::Error> {
    sqlx::query_as(&format!(
        "SELECT {SLIP_COLUMNS} FROM deposit_slips WHERE tenant_id = $1 AND id = $2"
    ))
    .bind(tenant_id)
    .bind(id)
    .fetch_optional(&mut **tx)
    .await
}

fn channel_label(channel: &str) -> &'static str {
    match channel {
        "pos_terminal" => "POS terminal",
        "cash_slip" => "cash paying-in slip",
        "card_to_card" => "card-to-card transfer",
        "wire_transfer" => "PAYA/SATNA wire transfer",
        _ => "deposit",
    }
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

async fn post_two_line_voucher(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: i64,
    fiscal_year_id: i64,
    voucher_date: NaiveDate,
    narration: &str,
    source_kind: i16,
    source_id: i64,
    debit_account_id: i64,
    credit_account_id: i64,
    amount: i64,
    actor_id: i64,
) -> Result<i64, (StatusCode, Json<Value>)> {
    let lines = [
        GeneratedLine {
            account_id: debit_account_id,
            debit: amount,
            credit: 0,
            description: narration.to_string(),
        },
        GeneratedLine {
            account_id: credit_account_id,
            debit: 0,
            credit: amount,
            description: narration.to_string(),
        },
    ];
    auto_post::post_generated_voucher(
        tx,
        tenant_id,
        fiscal_year_id,
        voucher_date,
        narration,
        source_kind,
        source_id,
        &lines,
        actor_id,
    )
    .await
    .map_err(posting_error_response)
}

fn posting_error_response(err: PostingError) -> (StatusCode, Json<Value>) {
    match err {
        PostingError::AccountNotFound(_) => not_found("account"),
        PostingError::AccountNotLeaf(_) => bad_request("account_not_leaf"),
        PostingError::FiscalYearNotFound => not_found("fiscal_year"),
        PostingError::FiscalYearClosed => bad_request("fiscal_year_closed"),
        PostingError::EmptyLines
        | PostingError::Unbalanced
        | PostingError::InvalidLineAmount(_)
        | PostingError::Database(_) => internal_error(),
    }
}

async fn replace_voucher(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: i64,
    old_voucher_id: i64,
) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM vouchers WHERE tenant_id = $1 AND id = $2")
        .bind(tenant_id)
        .bind(old_voucher_id)
        .execute(&mut **tx)
        .await?;
    Ok(())
}

// ---- create / update -------------------------------------------------------

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SlipFields {
    fiscal_year_id: i64,
    slip_number: Option<String>,
    slip_date: NaiveDate,
    amount: i64,
    description: Option<String>,
    payer_account_id: i64,
    bank_account_id: i64,
    channel: String,
    /// Step 5.7: attaches this deposit slip to an inventory invoice at creation time — see
    /// `received_cheques.rs::ReceiveFields::source_document_id`'s doc comment for the full
    /// rationale (same mechanism, same `source_module = 1` convention).
    #[serde(default)]
    source_document_id: Option<i64>,
}

const VALID_CHANNELS: &[&str] = &["pos_terminal", "cash_slip", "card_to_card", "wire_transfer"];

async fn create_slip(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<SlipFields>,
) -> Result<(StatusCode, Json<Value>), (StatusCode, Json<Value>)> {
    if req.amount <= 0 {
        return Err(bad_request("amount_must_be_positive"));
    }
    if !VALID_CHANNELS.contains(&req.channel.as_str()) {
        return Err(bad_request("invalid_channel"));
    }

    let mut tx = db::begin(&state.pool, auth.tenant_id)
        .await
        .map_err(|_| internal_error())?;
    fiscal_year_gate(&mut tx, auth.tenant_id, req.fiscal_year_id, req.slip_date).await?;
    require_leaf_account(&mut tx, auth.tenant_id, req.payer_account_id).await?;
    require_leaf_account(&mut tx, auth.tenant_id, req.bank_account_id).await?;
    if let Some(document_id) = req.source_document_id {
        let exists: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM inventory_documents WHERE tenant_id = $1 AND id = $2)",
        )
        .bind(auth.tenant_id)
        .bind(document_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(|_| internal_error())?;
        if !exists {
            return Err(bad_request("source_document_not_found"));
        }
    }
    let source_module: i16 = if req.source_document_id.is_some() {
        1
    } else {
        0
    };

    let id: i64 = sqlx::query_scalar(
        "INSERT INTO deposit_slips \
         (tenant_id, fiscal_year_id, slip_number, slip_date, amount, description, payer_account_id, \
          bank_account_id, channel, source_module, source_id, created_by) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9::deposit_channel, $10, $11, $12) RETURNING id",
    )
    .bind(auth.tenant_id)
    .bind(req.fiscal_year_id)
    .bind(&req.slip_number)
    .bind(req.slip_date)
    .bind(req.amount)
    .bind(&req.description)
    .bind(req.payer_account_id)
    .bind(req.bank_account_id)
    .bind(&req.channel)
    .bind(source_module)
    .bind(req.source_document_id)
    .bind(auth.user_id)
    .fetch_one(&mut *tx)
    .await
    .map_err(|_| internal_error())?;

    // §6.4: debit the bank/cash account, credit the payer -- one correct
    // narration for both lines (the structural fix for the swap defect).
    let narration = req.description.clone().unwrap_or_else(|| {
        format!(
            "Deposit via {} -- slip {}",
            channel_label(&req.channel),
            req.slip_number.clone().unwrap_or_default()
        )
    });
    let voucher_id = post_two_line_voucher(
        &mut tx,
        auth.tenant_id,
        req.fiscal_year_id,
        req.slip_date,
        &narration,
        25, // deposit_slip
        id,
        req.bank_account_id,
        req.payer_account_id,
        req.amount,
        auth.user_id,
    )
    .await?;
    sqlx::query("UPDATE deposit_slips SET voucher_id = $1 WHERE id = $2")
        .bind(voucher_id)
        .bind(id)
        .execute(&mut *tx)
        .await
        .map_err(|_| internal_error())?;

    audit::record_mutation(
        &mut tx,
        auth.tenant_id,
        "deposit_slips",
        id,
        "insert",
        Some(auth.user_id),
        None,
        Some(json!({ "amount": req.amount, "slipNumber": req.slip_number })),
    )
    .await
    .map_err(|_| internal_error())?;

    tx.commit().await.map_err(|_| internal_error())?;
    Ok((StatusCode::CREATED, Json(json!({ "id": id }))))
}

async fn update_slip(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
    Json(req): Json<SlipFields>,
) -> Result<StatusCode, (StatusCode, Json<Value>)> {
    if req.amount <= 0 {
        return Err(bad_request("amount_must_be_positive"));
    }
    if !VALID_CHANNELS.contains(&req.channel.as_str()) {
        return Err(bad_request("invalid_channel"));
    }

    let mut tx = db::begin(&state.pool, auth.tenant_id)
        .await
        .map_err(|_| internal_error())?;
    let Some(existing) = fetch_slip(&mut tx, auth.tenant_id, id)
        .await
        .map_err(|_| internal_error())?
    else {
        return Err(not_found("deposit_slip"));
    };
    fiscal_year_gate(&mut tx, auth.tenant_id, req.fiscal_year_id, req.slip_date).await?;
    require_leaf_account(&mut tx, auth.tenant_id, req.payer_account_id).await?;
    require_leaf_account(&mut tx, auth.tenant_id, req.bank_account_id).await?;

    sqlx::query(
        "UPDATE deposit_slips SET fiscal_year_id = $1, slip_number = $2, slip_date = $3, amount = $4, \
         description = $5, payer_account_id = $6, bank_account_id = $7, channel = $8::deposit_channel, \
         voucher_id = NULL, updated_at = now(), updated_by = $9 WHERE id = $10",
    )
    .bind(req.fiscal_year_id)
    .bind(&req.slip_number)
    .bind(req.slip_date)
    .bind(req.amount)
    .bind(&req.description)
    .bind(req.payer_account_id)
    .bind(req.bank_account_id)
    .bind(&req.channel)
    .bind(auth.user_id)
    .bind(id)
    .execute(&mut *tx)
    .await
    .map_err(|_| internal_error())?;

    if let Some(old_voucher_id) = existing.voucher_id {
        replace_voucher(&mut tx, auth.tenant_id, old_voucher_id)
            .await
            .map_err(|_| internal_error())?;
    }
    let narration = req.description.clone().unwrap_or_else(|| {
        format!(
            "Deposit via {} -- slip {}",
            channel_label(&req.channel),
            req.slip_number.clone().unwrap_or_default()
        )
    });
    let voucher_id = post_two_line_voucher(
        &mut tx,
        auth.tenant_id,
        req.fiscal_year_id,
        req.slip_date,
        &narration,
        25,
        id,
        req.bank_account_id,
        req.payer_account_id,
        req.amount,
        auth.user_id,
    )
    .await?;
    sqlx::query("UPDATE deposit_slips SET voucher_id = $1 WHERE id = $2")
        .bind(voucher_id)
        .bind(id)
        .execute(&mut *tx)
        .await
        .map_err(|_| internal_error())?;

    audit::record_mutation(
        &mut tx,
        auth.tenant_id,
        "deposit_slips",
        id,
        "update",
        Some(auth.user_id),
        Some(json!({ "amount": existing.amount })),
        Some(json!({ "amount": req.amount })),
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

async fn list_slips(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(params): Query<ListQuery>,
) -> Result<Json<Vec<SlipRecord>>, (StatusCode, Json<Value>)> {
    let mut tx = db::begin(&state.pool, auth.tenant_id)
        .await
        .map_err(|_| internal_error())?;
    let rows: Vec<SlipRecord> = if let Some(fy) = params.fiscal_year_id {
        sqlx::query_as(&format!(
            "SELECT {SLIP_COLUMNS} FROM deposit_slips WHERE tenant_id = $1 AND fiscal_year_id = $2 ORDER BY slip_date"
        ))
        .bind(auth.tenant_id)
        .bind(fy)
        .fetch_all(&mut *tx)
        .await
        .map_err(|_| internal_error())?
    } else {
        sqlx::query_as(&format!(
            "SELECT {SLIP_COLUMNS} FROM deposit_slips WHERE tenant_id = $1 ORDER BY slip_date"
        ))
        .bind(auth.tenant_id)
        .fetch_all(&mut *tx)
        .await
        .map_err(|_| internal_error())?
    };
    tx.rollback().await.ok();
    Ok(Json(rows))
}

async fn get_slip(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<Json<SlipRecord>, (StatusCode, Json<Value>)> {
    let mut tx = db::begin(&state.pool, auth.tenant_id)
        .await
        .map_err(|_| internal_error())?;
    let Some(slip) = fetch_slip(&mut tx, auth.tenant_id, id)
        .await
        .map_err(|_| internal_error())?
    else {
        return Err(not_found("deposit_slip"));
    };
    tx.rollback().await.ok();
    Ok(Json(slip))
}

// ---- delete -------------------------------------------------------------

async fn delete_slip(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<StatusCode, (StatusCode, Json<Value>)> {
    let mut tx = db::begin(&state.pool, auth.tenant_id)
        .await
        .map_err(|_| internal_error())?;
    let Some(slip) = fetch_slip(&mut tx, auth.tenant_id, id)
        .await
        .map_err(|_| internal_error())?
    else {
        return Err(not_found("deposit_slip"));
    };
    if slip.source_module != 0 {
        return Err(bad_request("linked_document_use_source_module"));
    }
    if let Some(voucher_id) = slip.voucher_id {
        let status: Option<String> = sqlx::query_scalar(
            "SELECT status::text FROM vouchers WHERE tenant_id = $1 AND id = $2",
        )
        .bind(auth.tenant_id)
        .bind(voucher_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|_| internal_error())?;
        if status.as_deref() != Some("draft") {
            return Err(bad_request("voucher_not_draft"));
        }
    }

    sqlx::query("DELETE FROM deposit_slips WHERE tenant_id = $1 AND id = $2")
        .bind(auth.tenant_id)
        .bind(id)
        .execute(&mut *tx)
        .await
        .map_err(|_| internal_error())?;
    if let Some(voucher_id) = slip.voucher_id {
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
        "deposit_slips",
        id,
        "delete",
        Some(auth.user_id),
        Some(json!({ "amount": slip.amount, "slipNumber": slip.slip_number })),
        None,
    )
    .await
    .map_err(|_| internal_error())?;

    tx.commit().await.map_err(|_| internal_error())?;
    Ok(StatusCode::NO_CONTENT)
}
