//! Step 4.1 (docs/phase-4-treasury.md §4.1): the received-cheque lifecycle
//! (legacy `DCheck`/`DCheck2`), with the B11 state-code ambiguity fixed by
//! construction — specs/06-treasury/06-01-entity-model.md §1.1-§1.2 and
//! 06-02-cheque-state-machine.md are the behavioural ground truth.
//!
//! **B11 fix**: `cheque_status` (migration 0011) has a distinct value for
//! every real state — `InHand`, `AtBank`, `Bounced`, `ReturnedToIssuer`,
//! `Cleared` — instead of the legacy's `S_State=1` meaning both
//! "never deposited" and "deposited then bounced" (§2.1). `Bounced` is now
//! genuinely reachable and queryable as its own state.
//!
//! **Event log is complete from day one** (§2.0's "the history of a cheque
//! always starts at its second event" is not reproduced): every transition
//! below, including receipt (T1), appends a `received_cheque_events` row.
//!
//! Step 4.2 (docs/phase-4-treasury.md §4.2): every transition below now posts
//! through the Phase 2.5 engine (`auto_post::post_generated_voucher`) inside
//! the SAME transaction as the state change — the cheque, its event row and
//! its voucher commit together or not at all.
//!
//! **B13 fix**: collection (T6) posts through the identical
//! `post_transition_voucher` path as every other transition, so "the only
//! treasury screen with no `DMoein_Make` call" (06-08.md §8.5 defect 1)
//! cannot recur structurally — there is no separate code path for T6 that
//! could omit the header build.
//!
//! **B10 fix**: `resulting_status` on every event row is derived from the
//! exact enum value written to the master row in the same statement group —
//! there is no way for the two to disagree the way the legacy's bounce
//! screen does (`S_State=2` on the event, `S_State=1` on the master,
//! 06-02.md §2.1).
//!
//! **B12 fix**: `DELETE /{id}` is a real operation — not the legacy's bare
//! `Exit;` above working code (`CheckListDU.pas:457`). Only a cheque that is
//! still `in_hand` with no event beyond its own receipt may be deleted;
//! deleting removes the cheque (cascading its lone event row), and its
//! receipt voucher, in one transaction. Anything else is rejected with a
//! clear reason, never a silent no-op.
//!
//! **One shared voucher per day is deliberately NOT reproduced** (06-08.md
//! §8.2's `Get_NewSanad_DateID` space-saving trick) — every transition gets
//! its own voucher via the Phase 2.5 engine's own atomic numbering.
//!
//! **`voucher_id` placement mirrors the legacy exactly**: `received_cheques.
//! voucher_id` (legacy `DCheck.S_Sanad`) is set ONLY by receipt (T1) and
//! amend (T2, which re-allocates it) — later transitions post to their own
//! voucher and record it only on their own `received_cheque_events.
//! voucher_id` row (legacy: "later transitions post to their own voucher
//! numbers, which are not stored on `DCheck`", §1.1).
//!
//! **Account resolution, a judgment call:** 06-02.md §2.3 T5 (bounce) and T6
//! (collect) describe accounts either "derived" from a per-tenant system
//! config (`system_accounts`, roles `cash`/`notes_in_collection`) crossed
//! with the payer's own Tafsil-1, or operator-picked with that derivation
//! only as a *default*. No step has ever populated `system_accounts` with
//! real rows (no tenant-provisioning flow exists yet — same gap 2.1/3.1
//! documented for `account_code_format`/`party_account_config`), so instead
//! of building that machinery now, this module keeps every account explicit
//! on the transition that actually chooses it (receipt, deposit, collect —
//! all operator-supplied, leaf-checked) and derives the two transitions the
//! spec says use no picker at all (bounce, return-to-issuer) from data
//! already on hand: bounce reverses the accounts of the cheque's own most
//! recent deposit event (behaviourally identical to re-deriving from
//! Tafsil-1, since that's exactly what the deposit's own accounts already
//! encode); return-to-issuer uses the cheque's own `payer_account_id`/
//! `notes_receivable_account_id`, exactly as the spec's T7 describes.
//!
//! Step 4.3 (docs/phase-4-treasury.md §4.3): a genuine endorsement feature —
//! the legacy reserves `S_Zssn`/`S_ZCR`/`S_ZName` for this and never builds
//! a screen, menu entry or voucher id around them (B14, 06-04-endorsement-
//! transfer-third-party.md). `EndorsedToThirdParty` (migration 0012) is a
//! new terminal state reachable from `InHand` or `Bounced`, structurally the
//! mirror of return-to-issuer (T7) except the DEBIT side is an
//! operator-chosen beneficiary account (leaf-checked, via the account
//! picker once 4.5 builds one) rather than the cheque's own fixed
//! `payer_account_id` — the credit side is still the cheque's own
//! `notes_receivable_account_id`, same as T7. This is new ground the spec
//! infers from the abandoned schema shape, not a port of working legacy
//! behaviour.

use crate::{
    audit,
    auth::AuthUser,
    auto_post::{self, GeneratedLine, PostingError},
    db, AppState,
};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::{Postgres, Transaction};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(list_cheques).post(receive_cheque))
        .route(
            "/{id}",
            get(get_cheque).put(amend_cheque).delete(delete_cheque),
        )
        .route("/{id}/deposit", post(deposit_to_bank))
        .route("/{id}/bounce", post(bounce_from_bank))
        .route("/{id}/collect", post(collect_cheque))
        .route("/{id}/return-to-issuer", post(return_to_issuer))
        .route("/{id}/endorse", post(endorse_to_third_party))
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

// ---- shared records --------------------------------------------------------

#[derive(sqlx::FromRow, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct ChequeRecord {
    id: i64,
    fiscal_year_id: i64,
    status: String,
    cheque_number: Option<String>,
    received_on: NaiveDate,
    due_date: NaiveDate,
    amount: i64,
    description: String,
    payer_account_id: i64,
    notes_receivable_account_id: i64,
    issuing_bank: Option<String>,
    issuing_branch: Option<String>,
    issuing_account_number: Option<String>,
    drawer_name: Option<String>,
    deposited_at: Option<NaiveDate>,
    cleared_at: Option<NaiveDate>,
    bounced_at: Option<NaiveDate>,
    returned_at: Option<NaiveDate>,
    endorsed_at: Option<NaiveDate>,
    voucher_id: Option<i64>,
}

const CHEQUE_COLUMNS: &str = "id, fiscal_year_id, status::text, cheque_number, received_on, \
    due_date, amount, description, payer_account_id, notes_receivable_account_id, issuing_bank, \
    issuing_branch, issuing_account_number, drawer_name, deposited_at, cleared_at, bounced_at, \
    returned_at, endorsed_at, voucher_id";

async fn fetch_cheque(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: i64,
    id: i64,
) -> Result<Option<ChequeRecord>, sqlx::Error> {
    sqlx::query_as(&format!(
        "SELECT {CHEQUE_COLUMNS} FROM received_cheques WHERE tenant_id = $1 AND id = $2"
    ))
    .bind(tenant_id)
    .bind(id)
    .fetch_optional(&mut **tx)
    .await
}

#[derive(sqlx::FromRow, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct EventRecord {
    id: i64,
    resulting_status: String,
    event_date: NaiveDate,
    amount: i64,
    debit_account_id: Option<i64>,
    credit_account_id: Option<i64>,
    description: Option<String>,
    voucher_id: Option<i64>,
}

const EVENT_COLUMNS: &str = "id, resulting_status::text, event_date, amount, debit_account_id, \
    credit_account_id, description, voucher_id";

async fn fetch_events(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: i64,
    cheque_id: i64,
) -> Result<Vec<EventRecord>, sqlx::Error> {
    sqlx::query_as(&format!(
        "SELECT {EVENT_COLUMNS} FROM received_cheque_events \
         WHERE tenant_id = $1 AND received_cheque_id = $2 ORDER BY id"
    ))
    .bind(tenant_id)
    .bind(cheque_id)
    .fetch_all(&mut **tx)
    .await
}

/// The most recent event that put the cheque `AtBank` — its accounts are
/// what bounce (reverse them) and collect (credit side) both reuse instead
/// of re-deriving from `system_accounts` (see module doc comment).
async fn latest_deposit_event(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: i64,
    cheque_id: i64,
) -> Result<Option<(i64, i64)>, sqlx::Error> {
    sqlx::query_as(
        "SELECT debit_account_id, credit_account_id FROM received_cheque_events \
         WHERE tenant_id = $1 AND received_cheque_id = $2 AND resulting_status = 'at_bank' \
         ORDER BY id DESC LIMIT 1",
    )
    .bind(tenant_id)
    .bind(cheque_id)
    .fetch_optional(&mut **tx)
    .await
    .map(|row: Option<(Option<i64>, Option<i64>)>| {
        row.and_then(|(d, c)| match (d, c) {
            (Some(d), Some(c)) => Some((d, c)),
            _ => None,
        })
    })
}

async fn insert_event(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: i64,
    cheque_id: i64,
    fiscal_year_id: i64,
    resulting_status: &str,
    event_date: NaiveDate,
    amount: i64,
    debit_account_id: Option<i64>,
    credit_account_id: Option<i64>,
    description: Option<&str>,
    actor_id: i64,
) -> Result<i64, sqlx::Error> {
    sqlx::query_scalar(
        "INSERT INTO received_cheque_events \
         (tenant_id, received_cheque_id, fiscal_year_id, resulting_status, event_date, amount, \
          debit_account_id, credit_account_id, description, created_by) \
         VALUES ($1, $2, $3, $4::cheque_status, $5, $6, $7, $8, $9, $10) RETURNING id",
    )
    .bind(tenant_id)
    .bind(cheque_id)
    .bind(fiscal_year_id)
    .bind(resulting_status)
    .bind(event_date)
    .bind(amount)
    .bind(debit_account_id)
    .bind(credit_account_id)
    .bind(description)
    .bind(actor_id)
    .fetch_one(&mut **tx)
    .await
}

/// Posts a two-line debit/credit voucher through the Phase 2.5 engine and
/// returns its id. Every account here has already been leaf-checked and
/// every fiscal year already gate-checked by the caller (`require_leaf_
/// account`/`fiscal_year_gate` above), so a `PostingError` surfacing here
/// would mean this module's own guards missed something — treated as an
/// internal error, not a user-facing validation failure.
async fn post_transition_voucher(
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
    .map_err(|err| match err {
        PostingError::AccountNotFound(_) => not_found("account"),
        PostingError::AccountNotLeaf(_) => bad_request("account_not_leaf"),
        PostingError::FiscalYearNotFound => not_found("fiscal_year"),
        PostingError::FiscalYearClosed => bad_request("fiscal_year_closed"),
        PostingError::EmptyLines
        | PostingError::Unbalanced
        | PostingError::InvalidLineAmount(_)
        | PostingError::Database(_) => internal_error(),
    })
}

/// Detaches every event referencing `voucher_id` (so the FK doesn't block
/// the delete) and deletes the voucher itself (cascading its lines) —
/// step T2 (amend) re-posts a fresh receipt voucher, mirroring the legacy's
/// "reallocates the voucher number from the date on every save" (§2.3 T2).
async fn replace_voucher(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: i64,
    cheque_id: i64,
    old_voucher_id: i64,
) -> Result<(), sqlx::Error> {
    // The cheque's own row references `old_voucher_id` too (it's the one
    // being amended) — detach it first, or the FK blocks the delete below.
    sqlx::query("UPDATE received_cheques SET voucher_id = NULL WHERE tenant_id = $1 AND id = $2")
        .bind(tenant_id)
        .bind(cheque_id)
        .execute(&mut **tx)
        .await?;
    sqlx::query("UPDATE received_cheque_events SET voucher_id = NULL WHERE tenant_id = $1 AND voucher_id = $2")
        .bind(tenant_id)
        .bind(old_voucher_id)
        .execute(&mut **tx)
        .await?;
    sqlx::query("DELETE FROM vouchers WHERE tenant_id = $1 AND id = $2")
        .bind(tenant_id)
        .bind(old_voucher_id)
        .execute(&mut **tx)
        .await?;
    Ok(())
}

// ---- shared validation ------------------------------------------------------

/// V-checks from 06-02.md §2.3 T1's field rules: amount > 0 (also DB-
/// enforced), description non-blank.
fn validate_amount_and_description(amount: i64, description: &str) -> Result<(), &'static str> {
    if amount <= 0 {
        return Err("amount_must_be_positive");
    }
    if description.trim().is_empty() {
        return Err("description_required");
    }
    Ok(())
}

/// Leaf-only, matching `Dm.is_Sarfasl_Last_Deep_SSN` (§2.3 T1). Returns
/// `AccountNotFound` (404-shaped) vs `AccountNotLeaf` (400-shaped) so callers
/// can render the right status.
enum AccountError {
    NotFound,
    NotLeaf,
    Db,
}
impl From<sqlx::Error> for AccountError {
    fn from(_: sqlx::Error) -> Self {
        AccountError::Db
    }
}
async fn require_leaf_account(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: i64,
    account_id: i64,
) -> Result<(), AccountError> {
    let child_count: Option<i32> =
        sqlx::query_scalar("SELECT child_count FROM accounts WHERE tenant_id = $1 AND id = $2")
            .bind(tenant_id)
            .bind(account_id)
            .fetch_optional(&mut **tx)
            .await?;
    match child_count {
        None => Err(AccountError::NotFound),
        Some(c) if c > 0 => Err(AccountError::NotLeaf),
        _ => Ok(()),
    }
}
fn account_error_response(err: AccountError) -> (StatusCode, Json<Value>) {
    match err {
        AccountError::NotFound => not_found("account"),
        AccountError::NotLeaf => bad_request("account_not_leaf"),
        AccountError::Db => internal_error(),
    }
}

/// A transition's own fiscal year (which may differ from the cheque's
/// receipt year, §1.2) must be open and contain `event_date`, and must not
/// be *earlier* than the year the cheque was received in (§2.3 T4/T5/T6/T7's
/// `S_COID <= Dm.CO_ID` — you may act in a later year, never an earlier one).
enum FiscalYearGateError {
    NotFound,
    Closed,
    DateOutsideRange,
    PrecedesReceiptYear,
    Db,
}
impl From<sqlx::Error> for FiscalYearGateError {
    fn from(_: sqlx::Error) -> Self {
        FiscalYearGateError::Db
    }
}
async fn fiscal_year_gate(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: i64,
    event_fiscal_year_id: i64,
    event_date: NaiveDate,
    received_fiscal_year_id: i64,
) -> Result<(), FiscalYearGateError> {
    let year: Option<(bool, NaiveDate, NaiveDate)> = sqlx::query_as(
        "SELECT is_active, start_date, end_date FROM fiscal_years WHERE tenant_id = $1 AND id = $2",
    )
    .bind(tenant_id)
    .bind(event_fiscal_year_id)
    .fetch_optional(&mut **tx)
    .await?;
    let Some((is_active, start, end)) = year else {
        return Err(FiscalYearGateError::NotFound);
    };
    if !is_active {
        return Err(FiscalYearGateError::Closed);
    }
    if event_date < start || event_date > end {
        return Err(FiscalYearGateError::DateOutsideRange);
    }
    if event_fiscal_year_id != received_fiscal_year_id {
        let received_start: Option<NaiveDate> = sqlx::query_scalar(
            "SELECT start_date FROM fiscal_years WHERE tenant_id = $1 AND id = $2",
        )
        .bind(tenant_id)
        .bind(received_fiscal_year_id)
        .fetch_optional(&mut **tx)
        .await?;
        if let Some(received_start) = received_start {
            if start < received_start {
                return Err(FiscalYearGateError::PrecedesReceiptYear);
            }
        }
    }
    Ok(())
}
fn fiscal_year_gate_response(err: FiscalYearGateError) -> (StatusCode, Json<Value>) {
    match err {
        FiscalYearGateError::NotFound => not_found("fiscal_year"),
        FiscalYearGateError::Closed => bad_request("fiscal_year_closed"),
        FiscalYearGateError::DateOutsideRange => bad_request("date_outside_fiscal_year"),
        FiscalYearGateError::PrecedesReceiptYear => {
            bad_request("fiscal_year_precedes_receipt_year")
        }
        FiscalYearGateError::Db => internal_error(),
    }
}

// ---- T1: receive -----------------------------------------------------------

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ReceiveFields {
    fiscal_year_id: i64,
    cheque_number: Option<String>,
    received_on: NaiveDate,
    due_date: NaiveDate,
    amount: i64,
    description: String,
    payer_account_id: i64,
    notes_receivable_account_id: i64,
    issuing_bank: Option<String>,
    issuing_branch: Option<String>,
    issuing_account_number: Option<String>,
    drawer_name: Option<String>,
    /// Step 5.7 (docs/phase-5-inventory.md §5.7): attaches this cheque to an inventory invoice at
    /// receipt time, mirroring the legacy's `New_From_PRg(_Prg=1, _Factor, ...)` pre-filled-link
    /// flow (05-09-b.md §9.4.1) — the settlement link is set once, at creation, not retrofitted.
    /// Maps to `source_module = 1` ("inventory_invoice", already seeded by 2.3) and
    /// `source_id = <inventory_documents.id>`, the real surrogate-key link 05-09-b.md §9.7 point 1
    /// asks for (never the legacy's mutable invoice-number link).
    #[serde(default)]
    source_document_id: Option<i64>,
}

async fn receive_cheque(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<ReceiveFields>,
) -> Result<(StatusCode, Json<Value>), (StatusCode, Json<Value>)> {
    validate_amount_and_description(req.amount, &req.description).map_err(bad_request)?;

    let mut tx = db::begin(&state.pool, auth.tenant_id)
        .await
        .map_err(|_| internal_error())?;

    let year: Option<(bool, NaiveDate, NaiveDate)> = sqlx::query_as(
        "SELECT is_active, start_date, end_date FROM fiscal_years WHERE tenant_id = $1 AND id = $2",
    )
    .bind(auth.tenant_id)
    .bind(req.fiscal_year_id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|_| internal_error())?;
    let Some((is_active, start, end)) = year else {
        return Err(not_found("fiscal_year"));
    };
    if !is_active {
        return Err(bad_request("fiscal_year_closed"));
    }
    if req.received_on < start || req.received_on > end {
        return Err(bad_request("date_outside_fiscal_year"));
    }

    require_leaf_account(&mut tx, auth.tenant_id, req.payer_account_id)
        .await
        .map_err(account_error_response)?;
    require_leaf_account(&mut tx, auth.tenant_id, req.notes_receivable_account_id)
        .await
        .map_err(account_error_response)?;
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
        "INSERT INTO received_cheques \
         (tenant_id, fiscal_year_id, status, cheque_number, received_on, due_date, amount, \
          description, payer_account_id, notes_receivable_account_id, issuing_bank, issuing_branch, \
          issuing_account_number, drawer_name, source_module, source_id, created_by) \
         VALUES ($1, $2, 'in_hand', $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16) \
         RETURNING id",
    )
    .bind(auth.tenant_id)
    .bind(req.fiscal_year_id)
    .bind(&req.cheque_number)
    .bind(req.received_on)
    .bind(req.due_date)
    .bind(req.amount)
    .bind(&req.description)
    .bind(req.payer_account_id)
    .bind(req.notes_receivable_account_id)
    .bind(&req.issuing_bank)
    .bind(&req.issuing_branch)
    .bind(&req.issuing_account_number)
    .bind(&req.drawer_name)
    .bind(source_module)
    .bind(req.source_document_id)
    .bind(auth.user_id)
    .fetch_one(&mut *tx)
    .await
    .map_err(|_| internal_error())?;

    // Unlike the legacy (§2.0), a row is written for the receipt itself.
    let event_id = insert_event(
        &mut tx,
        auth.tenant_id,
        id,
        req.fiscal_year_id,
        "in_hand",
        req.received_on,
        req.amount,
        Some(req.notes_receivable_account_id),
        Some(req.payer_account_id),
        Some(&req.description),
        auth.user_id,
    )
    .await
    .map_err(|_| internal_error())?;

    // §2.3 T1's voucher: M_Id=21, debit notes-receivable, credit the payer.
    let voucher_id = post_transition_voucher(
        &mut tx,
        auth.tenant_id,
        req.fiscal_year_id,
        req.received_on,
        &req.description,
        21, // cheque_received
        id,
        req.notes_receivable_account_id,
        req.payer_account_id,
        req.amount,
        auth.user_id,
    )
    .await?;
    sqlx::query("UPDATE received_cheques SET voucher_id = $1 WHERE id = $2")
        .bind(voucher_id)
        .bind(id)
        .execute(&mut *tx)
        .await
        .map_err(|_| internal_error())?;
    sqlx::query("UPDATE received_cheque_events SET voucher_id = $1 WHERE id = $2")
        .bind(voucher_id)
        .bind(event_id)
        .execute(&mut *tx)
        .await
        .map_err(|_| internal_error())?;

    audit::record_mutation(
        &mut tx,
        auth.tenant_id,
        "received_cheques",
        id,
        "insert",
        Some(auth.user_id),
        None,
        Some(json!({ "amount": req.amount, "chequeNumber": req.cheque_number })),
    )
    .await
    .map_err(|_| internal_error())?;

    tx.commit().await.map_err(|_| internal_error())?;
    Ok((StatusCode::CREATED, Json(json!({ "id": id }))))
}

// ---- T2: amend (in-hand only) ----------------------------------------------

async fn amend_cheque(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
    Json(req): Json<ReceiveFields>,
) -> Result<StatusCode, (StatusCode, Json<Value>)> {
    validate_amount_and_description(req.amount, &req.description).map_err(bad_request)?;

    let mut tx = db::begin(&state.pool, auth.tenant_id)
        .await
        .map_err(|_| internal_error())?;
    let Some(existing) = fetch_cheque(&mut tx, auth.tenant_id, id)
        .await
        .map_err(|_| internal_error())?
    else {
        return Err(not_found("cheque"));
    };
    if existing.status != "in_hand" {
        return Err(bad_request("cheque_not_in_hand"));
    }
    // §2.3 T2: the fiscal year is not changeable on edit — you must be
    // acting within the year the cheque was received.
    let year: Option<(bool, NaiveDate, NaiveDate)> = sqlx::query_as(
        "SELECT is_active, start_date, end_date FROM fiscal_years WHERE tenant_id = $1 AND id = $2",
    )
    .bind(auth.tenant_id)
    .bind(existing.fiscal_year_id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|_| internal_error())?;
    let Some((is_active, start, end)) = year else {
        return Err(internal_error()); // the cheque's own year row must exist
    };
    if !is_active {
        return Err(bad_request("fiscal_year_closed"));
    }
    if req.received_on < start || req.received_on > end {
        return Err(bad_request("date_outside_fiscal_year"));
    }

    require_leaf_account(&mut tx, auth.tenant_id, req.payer_account_id)
        .await
        .map_err(account_error_response)?;
    require_leaf_account(&mut tx, auth.tenant_id, req.notes_receivable_account_id)
        .await
        .map_err(account_error_response)?;

    sqlx::query(
        "UPDATE received_cheques SET cheque_number = $1, received_on = $2, due_date = $3, \
         amount = $4, description = $5, payer_account_id = $6, notes_receivable_account_id = $7, \
         issuing_bank = $8, issuing_branch = $9, issuing_account_number = $10, drawer_name = $11, \
         updated_at = now(), updated_by = $12 WHERE id = $13",
    )
    .bind(&req.cheque_number)
    .bind(req.received_on)
    .bind(req.due_date)
    .bind(req.amount)
    .bind(&req.description)
    .bind(req.payer_account_id)
    .bind(req.notes_receivable_account_id)
    .bind(&req.issuing_bank)
    .bind(&req.issuing_branch)
    .bind(&req.issuing_account_number)
    .bind(&req.drawer_name)
    .bind(auth.user_id)
    .bind(id)
    .execute(&mut *tx)
    .await
    .map_err(|_| internal_error())?;

    // §2.3 T2: reallocates the voucher number and reposts the M_Id=21 lines —
    // delete the old receipt voucher, post a fresh one.
    if let Some(old_voucher_id) = existing.voucher_id {
        replace_voucher(&mut tx, auth.tenant_id, id, old_voucher_id)
            .await
            .map_err(|_| internal_error())?;
    }
    let voucher_id = post_transition_voucher(
        &mut tx,
        auth.tenant_id,
        existing.fiscal_year_id,
        req.received_on,
        &req.description,
        21, // cheque_received
        id,
        req.notes_receivable_account_id,
        req.payer_account_id,
        req.amount,
        auth.user_id,
    )
    .await?;
    sqlx::query("UPDATE received_cheques SET voucher_id = $1 WHERE id = $2")
        .bind(voucher_id)
        .bind(id)
        .execute(&mut *tx)
        .await
        .map_err(|_| internal_error())?;
    sqlx::query(
        "UPDATE received_cheque_events SET voucher_id = $1 \
         WHERE tenant_id = $2 AND received_cheque_id = $3 AND resulting_status = 'in_hand'::cheque_status",
    )
    .bind(voucher_id)
    .bind(auth.tenant_id)
    .bind(id)
    .execute(&mut *tx)
    .await
    .map_err(|_| internal_error())?;

    audit::record_mutation(
        &mut tx,
        auth.tenant_id,
        "received_cheques",
        id,
        "update",
        Some(auth.user_id),
        Some(json!({ "amount": existing.amount, "description": existing.description })),
        Some(json!({ "amount": req.amount, "description": req.description })),
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
    status: Option<String>,
    /// Step 4.5's aging view (06-05-due-date-logic.md §5.4b): "every cheque
    /// whose due date has arrived, that has not yet left the pipeline" —
    /// `due_date <= dueBefore` AND status excludes every terminal exit
    /// (`returned_to_issuer`/`cleared`/`endorsed_to_third_party`), unlike
    /// the legacy where this filter exists in the query but is unreachable
    /// from the UI at all (§5.5) — here it's a real, wired parameter.
    due_before: Option<NaiveDate>,
    cheque_number: Option<String>,
    description: Option<String>,
    sort: Option<String>,
    order: Option<String>,
}

const CHEQUE_SORT_COLUMNS: &[(&str, &str)] = &[
    ("status", "status::text"),
    ("chequeNumber", "cheque_number"),
    ("receivedOn", "received_on"),
    ("dueDate", "due_date"),
    ("amount", "amount"),
    ("description", "description"),
];

async fn list_cheques(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(params): Query<ListQuery>,
) -> Result<Json<Vec<ChequeRecord>>, (StatusCode, Json<Value>)> {
    let mut tx = db::begin(&state.pool, auth.tenant_id)
        .await
        .map_err(|_| internal_error())?;
    let rows: Vec<ChequeRecord> = sqlx::query_as(&format!(
        "SELECT {CHEQUE_COLUMNS} FROM received_cheques \
         WHERE tenant_id = $1 \
         AND ($2::bigint IS NULL OR fiscal_year_id = $2) \
         AND ($3::text IS NULL OR status = $3::cheque_status) \
         AND ($4::date IS NULL OR (due_date <= $4 AND status IN ('in_hand', 'at_bank', 'bounced'))) \
         AND ($5::text IS NULL OR cheque_number ILIKE '%' || $5 || '%') \
         AND ($6::text IS NULL OR description ILIKE '%' || $6 || '%') \
         ORDER BY {}",
        crate::sort::order_by(params.sort.as_deref(), params.order.as_deref(), CHEQUE_SORT_COLUMNS, "due_date"),
    ))
    .bind(auth.tenant_id)
    .bind(params.fiscal_year_id)
    .bind(&params.status)
    .bind(params.due_before)
    .bind(&params.cheque_number)
    .bind(&params.description)
    .fetch_all(&mut *tx)
    .await
    .map_err(|_| internal_error())?;
    tx.rollback().await.ok();
    Ok(Json(rows))
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ChequeDetail {
    #[serde(flatten)]
    cheque: ChequeRecord,
    events: Vec<EventRecord>,
}

async fn get_cheque(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<Json<ChequeDetail>, (StatusCode, Json<Value>)> {
    let mut tx = db::begin(&state.pool, auth.tenant_id)
        .await
        .map_err(|_| internal_error())?;
    let Some(cheque) = fetch_cheque(&mut tx, auth.tenant_id, id)
        .await
        .map_err(|_| internal_error())?
    else {
        return Err(not_found("cheque"));
    };
    let events = fetch_events(&mut tx, auth.tenant_id, id)
        .await
        .map_err(|_| internal_error())?;
    tx.rollback().await.ok();
    Ok(Json(ChequeDetail { cheque, events }))
}

// ---- shared transition fields ------------------------------------------------

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct TransitionFields {
    fiscal_year_id: i64,
    event_date: NaiveDate,
    description: Option<String>,
}

// ---- T4: deposit to bank (in_hand|bounced -> at_bank) ------------------------

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct DepositFields {
    #[serde(flatten)]
    common: TransitionFields,
    collection_account_id: i64,
}

async fn deposit_to_bank(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
    Json(req): Json<DepositFields>,
) -> Result<Json<ChequeRecord>, (StatusCode, Json<Value>)> {
    let mut tx = db::begin(&state.pool, auth.tenant_id)
        .await
        .map_err(|_| internal_error())?;
    let Some(cheque) = fetch_cheque(&mut tx, auth.tenant_id, id)
        .await
        .map_err(|_| internal_error())?
    else {
        return Err(not_found("cheque"));
    };
    if cheque.status != "in_hand" && cheque.status != "bounced" {
        return Err(bad_request("cheque_not_depositable"));
    }
    fiscal_year_gate(
        &mut tx,
        auth.tenant_id,
        req.common.fiscal_year_id,
        req.common.event_date,
        cheque.fiscal_year_id,
    )
    .await
    .map_err(fiscal_year_gate_response)?;
    require_leaf_account(&mut tx, auth.tenant_id, req.collection_account_id)
        .await
        .map_err(account_error_response)?;

    // Debit the collection account, credit whichever account currently holds
    // the cheque's value (§2.3 T4: "old S_BedSSN" — the notes-receivable
    // account from receipt on a first deposit, or the on-hand account
    // recorded by the bounce that preceded a re-deposit).
    let credit_account_id = cheque.notes_receivable_account_id;

    sqlx::query(
        "UPDATE received_cheques SET status = 'at_bank', deposited_at = $1, updated_at = now(), \
         updated_by = $2 WHERE id = $3",
    )
    .bind(req.common.event_date)
    .bind(auth.user_id)
    .bind(id)
    .execute(&mut *tx)
    .await
    .map_err(|_| internal_error())?;

    let event_id = insert_event(
        &mut tx,
        auth.tenant_id,
        id,
        req.common.fiscal_year_id,
        "at_bank",
        req.common.event_date,
        cheque.amount,
        Some(req.collection_account_id),
        Some(credit_account_id),
        req.common.description.as_deref(),
        auth.user_id,
    )
    .await
    .map_err(|_| internal_error())?;

    // §2.3 T4's voucher: M_Id=22 (shared with T5's bounce reversal, matching
    // the legacy — both are the same document family), M_Link=this event's
    // id (not the cheque id, per the spec's explicit note).
    let narration = req.common.description.clone().unwrap_or_else(|| {
        format!(
            "Cheque {} deposited to bank",
            cheque.cheque_number.clone().unwrap_or_default()
        )
    });
    let voucher_id = post_transition_voucher(
        &mut tx,
        auth.tenant_id,
        req.common.fiscal_year_id,
        req.common.event_date,
        &narration,
        22, // cheque_bounced -- shared M_Id family with deposit, see module doc comment
        event_id,
        req.collection_account_id,
        credit_account_id,
        cheque.amount,
        auth.user_id,
    )
    .await?;
    sqlx::query("UPDATE received_cheque_events SET voucher_id = $1 WHERE id = $2")
        .bind(voucher_id)
        .bind(event_id)
        .execute(&mut *tx)
        .await
        .map_err(|_| internal_error())?;

    audit::record_mutation(
        &mut tx,
        auth.tenant_id,
        "received_cheques",
        id,
        "update",
        Some(auth.user_id),
        Some(json!({ "status": cheque.status })),
        Some(json!({ "status": "at_bank" })),
    )
    .await
    .map_err(|_| internal_error())?;

    let updated = fetch_cheque(&mut tx, auth.tenant_id, id)
        .await
        .map_err(|_| internal_error())?
        .ok_or_else(internal_error)?;
    tx.commit().await.map_err(|_| internal_error())?;
    Ok(Json(updated))
}

// ---- T5: bounce (at_bank -> bounced) ------------------------------------------

async fn bounce_from_bank(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
    Json(req): Json<TransitionFields>,
) -> Result<Json<ChequeRecord>, (StatusCode, Json<Value>)> {
    let mut tx = db::begin(&state.pool, auth.tenant_id)
        .await
        .map_err(|_| internal_error())?;
    let Some(cheque) = fetch_cheque(&mut tx, auth.tenant_id, id)
        .await
        .map_err(|_| internal_error())?
    else {
        return Err(not_found("cheque"));
    };
    if cheque.status != "at_bank" {
        return Err(bad_request("cheque_not_at_bank"));
    }
    fiscal_year_gate(
        &mut tx,
        auth.tenant_id,
        req.fiscal_year_id,
        req.event_date,
        cheque.fiscal_year_id,
    )
    .await
    .map_err(fiscal_year_gate_response)?;

    // Reverse the accounts of the deposit that put it at_bank (see module
    // doc comment) — debit the on-hand account, credit the collection
    // account, an exact reversal of T4.
    let Some((collection_account_id, on_hand_account_id)) =
        latest_deposit_event(&mut tx, auth.tenant_id, id)
            .await
            .map_err(|_| internal_error())?
    else {
        return Err(internal_error()); // an at_bank cheque must have a deposit event
    };

    sqlx::query(
        "UPDATE received_cheques SET status = 'bounced', bounced_at = $1, updated_at = now(), \
         updated_by = $2 WHERE id = $3",
    )
    .bind(req.event_date)
    .bind(auth.user_id)
    .bind(id)
    .execute(&mut *tx)
    .await
    .map_err(|_| internal_error())?;

    // B10 fix: this row's resulting_status is 'bounced', agreeing exactly
    // with the master row just written above — not the legacy's
    // "master=1, event=2" disagreement (§2.1).
    let event_id = insert_event(
        &mut tx,
        auth.tenant_id,
        id,
        req.fiscal_year_id,
        "bounced",
        req.event_date,
        cheque.amount,
        Some(on_hand_account_id),
        Some(collection_account_id),
        req.description.as_deref(),
        auth.user_id,
    )
    .await
    .map_err(|_| internal_error())?;

    // §2.3 T5's voucher: M_Id=22 (same family as T4), an exact reversal.
    let narration = req.description.clone().unwrap_or_else(|| {
        format!(
            "Cheque {} bounced from bank",
            cheque.cheque_number.clone().unwrap_or_default()
        )
    });
    let voucher_id = post_transition_voucher(
        &mut tx,
        auth.tenant_id,
        req.fiscal_year_id,
        req.event_date,
        &narration,
        22, // cheque_bounced
        event_id,
        on_hand_account_id,
        collection_account_id,
        cheque.amount,
        auth.user_id,
    )
    .await?;
    sqlx::query("UPDATE received_cheque_events SET voucher_id = $1 WHERE id = $2")
        .bind(voucher_id)
        .bind(event_id)
        .execute(&mut *tx)
        .await
        .map_err(|_| internal_error())?;

    audit::record_mutation(
        &mut tx,
        auth.tenant_id,
        "received_cheques",
        id,
        "update",
        Some(auth.user_id),
        Some(json!({ "status": cheque.status })),
        Some(json!({ "status": "bounced" })),
    )
    .await
    .map_err(|_| internal_error())?;

    let updated = fetch_cheque(&mut tx, auth.tenant_id, id)
        .await
        .map_err(|_| internal_error())?
        .ok_or_else(internal_error)?;
    tx.commit().await.map_err(|_| internal_error())?;
    Ok(Json(updated))
}

// ---- T6: collect / clear (at_bank -> cleared) ---------------------------------

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CollectFields {
    #[serde(flatten)]
    common: TransitionFields,
    bank_account_id: i64,
}

async fn collect_cheque(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
    Json(req): Json<CollectFields>,
) -> Result<Json<ChequeRecord>, (StatusCode, Json<Value>)> {
    let mut tx = db::begin(&state.pool, auth.tenant_id)
        .await
        .map_err(|_| internal_error())?;
    let Some(cheque) = fetch_cheque(&mut tx, auth.tenant_id, id)
        .await
        .map_err(|_| internal_error())?
    else {
        return Err(not_found("cheque"));
    };
    if cheque.status != "at_bank" {
        return Err(bad_request("cheque_not_at_bank"));
    }
    fiscal_year_gate(
        &mut tx,
        auth.tenant_id,
        req.common.fiscal_year_id,
        req.common.event_date,
        cheque.fiscal_year_id,
    )
    .await
    .map_err(fiscal_year_gate_response)?;
    require_leaf_account(&mut tx, auth.tenant_id, req.bank_account_id)
        .await
        .map_err(account_error_response)?;

    let Some((collection_account_id, _on_hand_account_id)) =
        latest_deposit_event(&mut tx, auth.tenant_id, id)
            .await
            .map_err(|_| internal_error())?
    else {
        return Err(internal_error());
    };

    sqlx::query(
        "UPDATE received_cheques SET status = 'cleared', cleared_at = $1, updated_at = now(), \
         updated_by = $2 WHERE id = $3",
    )
    .bind(req.common.event_date)
    .bind(auth.user_id)
    .bind(id)
    .execute(&mut *tx)
    .await
    .map_err(|_| internal_error())?;

    // Debit the chosen bank account, credit the collection account the
    // cheque was sitting in (§2.3 T6).
    let event_id = insert_event(
        &mut tx,
        auth.tenant_id,
        id,
        req.common.fiscal_year_id,
        "cleared",
        req.common.event_date,
        cheque.amount,
        Some(req.bank_account_id),
        Some(collection_account_id),
        req.common.description.as_deref(),
        auth.user_id,
    )
    .await
    .map_err(|_| internal_error())?;

    // §2.3 T6's voucher: M_Id=23. B13 fix — this is the SAME posting path as
    // every other transition, so the legacy's "collection is the only
    // treasury screen with no DMoein_Make call" defect cannot recur.
    let narration = req.common.description.clone().unwrap_or_else(|| {
        format!(
            "Cheque {} collected",
            cheque.cheque_number.clone().unwrap_or_default()
        )
    });
    let voucher_id = post_transition_voucher(
        &mut tx,
        auth.tenant_id,
        req.common.fiscal_year_id,
        req.common.event_date,
        &narration,
        23, // cheque_collected
        event_id,
        req.bank_account_id,
        collection_account_id,
        cheque.amount,
        auth.user_id,
    )
    .await?;
    sqlx::query("UPDATE received_cheque_events SET voucher_id = $1 WHERE id = $2")
        .bind(voucher_id)
        .bind(event_id)
        .execute(&mut *tx)
        .await
        .map_err(|_| internal_error())?;

    audit::record_mutation(
        &mut tx,
        auth.tenant_id,
        "received_cheques",
        id,
        "update",
        Some(auth.user_id),
        Some(json!({ "status": cheque.status })),
        Some(json!({ "status": "cleared" })),
    )
    .await
    .map_err(|_| internal_error())?;

    let updated = fetch_cheque(&mut tx, auth.tenant_id, id)
        .await
        .map_err(|_| internal_error())?
        .ok_or_else(internal_error)?;
    tx.commit().await.map_err(|_| internal_error())?;
    Ok(Json(updated))
}

// ---- T7: return to issuer (in_hand|bounced -> returned_to_issuer) --------------

async fn return_to_issuer(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
    Json(req): Json<TransitionFields>,
) -> Result<Json<ChequeRecord>, (StatusCode, Json<Value>)> {
    let mut tx = db::begin(&state.pool, auth.tenant_id)
        .await
        .map_err(|_| internal_error())?;
    let Some(cheque) = fetch_cheque(&mut tx, auth.tenant_id, id)
        .await
        .map_err(|_| internal_error())?
    else {
        return Err(not_found("cheque"));
    };
    if cheque.status != "in_hand" && cheque.status != "bounced" {
        return Err(bad_request("cheque_not_returnable"));
    }
    fiscal_year_gate(
        &mut tx,
        auth.tenant_id,
        req.fiscal_year_id,
        req.event_date,
        cheque.fiscal_year_id,
    )
    .await
    .map_err(fiscal_year_gate_response)?;

    sqlx::query(
        "UPDATE received_cheques SET status = 'returned_to_issuer', returned_at = $1, \
         updated_at = now(), updated_by = $2 WHERE id = $3",
    )
    .bind(req.event_date)
    .bind(auth.user_id)
    .bind(id)
    .execute(&mut *tx)
    .await
    .map_err(|_| internal_error())?;

    // §2.3 T7: debit the payer (they get their obligation back), credit the
    // notes-receivable account — an exact reversal of T1. Both loaded from
    // the cheque itself, no picker, matching the spec exactly.
    let event_id = insert_event(
        &mut tx,
        auth.tenant_id,
        id,
        req.fiscal_year_id,
        "returned_to_issuer",
        req.event_date,
        cheque.amount,
        Some(cheque.payer_account_id),
        Some(cheque.notes_receivable_account_id),
        req.description.as_deref(),
        auth.user_id,
    )
    .await
    .map_err(|_| internal_error())?;

    // §2.3 T7's voucher: M_Id=24.
    let narration = req.description.clone().unwrap_or_else(|| {
        format!(
            "Cheque {} returned to issuer",
            cheque.cheque_number.clone().unwrap_or_default()
        )
    });
    let voucher_id = post_transition_voucher(
        &mut tx,
        auth.tenant_id,
        req.fiscal_year_id,
        req.event_date,
        &narration,
        24, // cheque_returned
        event_id,
        cheque.payer_account_id,
        cheque.notes_receivable_account_id,
        cheque.amount,
        auth.user_id,
    )
    .await?;
    sqlx::query("UPDATE received_cheque_events SET voucher_id = $1 WHERE id = $2")
        .bind(voucher_id)
        .bind(event_id)
        .execute(&mut *tx)
        .await
        .map_err(|_| internal_error())?;

    audit::record_mutation(
        &mut tx,
        auth.tenant_id,
        "received_cheques",
        id,
        "update",
        Some(auth.user_id),
        Some(json!({ "status": cheque.status })),
        Some(json!({ "status": "returned_to_issuer" })),
    )
    .await
    .map_err(|_| internal_error())?;

    let updated = fetch_cheque(&mut tx, auth.tenant_id, id)
        .await
        .map_err(|_| internal_error())?
        .ok_or_else(internal_error)?;
    tx.commit().await.map_err(|_| internal_error())?;
    Ok(Json(updated))
}

// ---- T3: delete (in_hand, receipt-only -> gone) --------------------------------
//
// B12 fix (11-open-decisions.md): a real delete, unlike the legacy's bare
// `Exit;` above working-but-unreachable code (`CheckListDU.pas:457`). Only a
// cheque that has never left `in_hand` (its only event is its own receipt)
// may be deleted — anything with real transition history is rejected with a
// clear reason, never a silent no-op.

async fn delete_cheque(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<StatusCode, (StatusCode, Json<Value>)> {
    let mut tx = db::begin(&state.pool, auth.tenant_id)
        .await
        .map_err(|_| internal_error())?;
    let Some(cheque) = fetch_cheque(&mut tx, auth.tenant_id, id)
        .await
        .map_err(|_| internal_error())?
    else {
        return Err(not_found("cheque"));
    };
    if cheque.status != "in_hand" {
        return Err(bad_request("cheque_not_deletable"));
    }
    let events = fetch_events(&mut tx, auth.tenant_id, id)
        .await
        .map_err(|_| internal_error())?;
    if events.len() != 1 {
        return Err(bad_request("cheque_has_transition_history"));
    }

    // Cascades the lone receipt event automatically (ON DELETE CASCADE).
    sqlx::query("DELETE FROM received_cheques WHERE tenant_id = $1 AND id = $2")
        .bind(auth.tenant_id)
        .bind(id)
        .execute(&mut *tx)
        .await
        .map_err(|_| internal_error())?;
    if let Some(voucher_id) = cheque.voucher_id {
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
        "received_cheques",
        id,
        "delete",
        Some(auth.user_id),
        Some(json!({ "amount": cheque.amount, "chequeNumber": cheque.cheque_number })),
        None,
    )
    .await
    .map_err(|_| internal_error())?;

    tx.commit().await.map_err(|_| internal_error())?;
    Ok(StatusCode::NO_CONTENT)
}

// ---- T8: endorse to a third party (in_hand|bounced -> endorsed_to_third_party) -

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct EndorseFields {
    #[serde(flatten)]
    common: TransitionFields,
    beneficiary_account_id: i64,
}

/// Step 4.3: a genuine new feature (B14) — the legacy never built this.
/// Structurally the mirror of T7 (return-to-issuer): debit the chosen
/// beneficiary instead of the fixed payer, credit the cheque's own
/// `notes_receivable_account_id`, same as T7.
async fn endorse_to_third_party(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
    Json(req): Json<EndorseFields>,
) -> Result<Json<ChequeRecord>, (StatusCode, Json<Value>)> {
    let mut tx = db::begin(&state.pool, auth.tenant_id)
        .await
        .map_err(|_| internal_error())?;
    let Some(cheque) = fetch_cheque(&mut tx, auth.tenant_id, id)
        .await
        .map_err(|_| internal_error())?
    else {
        return Err(not_found("cheque"));
    };
    if cheque.status != "in_hand" && cheque.status != "bounced" {
        return Err(bad_request("cheque_not_endorsable"));
    }
    fiscal_year_gate(
        &mut tx,
        auth.tenant_id,
        req.common.fiscal_year_id,
        req.common.event_date,
        cheque.fiscal_year_id,
    )
    .await
    .map_err(fiscal_year_gate_response)?;
    require_leaf_account(&mut tx, auth.tenant_id, req.beneficiary_account_id)
        .await
        .map_err(account_error_response)?;

    sqlx::query(
        "UPDATE received_cheques SET status = 'endorsed_to_third_party', endorsed_at = $1, \
         updated_at = now(), updated_by = $2 WHERE id = $3",
    )
    .bind(req.common.event_date)
    .bind(auth.user_id)
    .bind(id)
    .execute(&mut *tx)
    .await
    .map_err(|_| internal_error())?;

    let event_id = insert_event(
        &mut tx,
        auth.tenant_id,
        id,
        req.common.fiscal_year_id,
        "endorsed_to_third_party",
        req.common.event_date,
        cheque.amount,
        Some(req.beneficiary_account_id),
        Some(cheque.notes_receivable_account_id),
        req.common.description.as_deref(),
        auth.user_id,
    )
    .await
    .map_err(|_| internal_error())?;

    let narration = req.common.description.clone().unwrap_or_else(|| {
        format!(
            "Cheque {} endorsed to third party",
            cheque.cheque_number.clone().unwrap_or_default()
        )
    });
    let voucher_id = post_transition_voucher(
        &mut tx,
        auth.tenant_id,
        req.common.fiscal_year_id,
        req.common.event_date,
        &narration,
        27, // cheque_endorsed
        event_id,
        req.beneficiary_account_id,
        cheque.notes_receivable_account_id,
        cheque.amount,
        auth.user_id,
    )
    .await?;
    sqlx::query("UPDATE received_cheque_events SET voucher_id = $1 WHERE id = $2")
        .bind(voucher_id)
        .bind(event_id)
        .execute(&mut *tx)
        .await
        .map_err(|_| internal_error())?;

    audit::record_mutation(
        &mut tx,
        auth.tenant_id,
        "received_cheques",
        id,
        "update",
        Some(auth.user_id),
        Some(json!({ "status": cheque.status })),
        Some(json!({ "status": "endorsed_to_third_party" })),
    )
    .await
    .map_err(|_| internal_error())?;

    let updated = fetch_cheque(&mut tx, auth.tenant_id, id)
        .await
        .map_err(|_| internal_error())?
        .ok_or_else(internal_error)?;
    tx.commit().await.map_err(|_| internal_error())?;
    Ok(Json(updated))
}
