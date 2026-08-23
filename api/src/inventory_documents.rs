//! Step 5.2 (docs/phase-5-inventory.md §5.2): purchase/sale/return invoice ("Factor") documents,
//! with a real explicit `status` column (specs/05-inventory/05-04-a.md §4.0's "the lifecycle is
//! entirely derived from three other tables" replaced outright) and the B7 counterparty fix
//! (`11-open-decisions.md`).
//!
//! Incremental per-line CRUD, same architectural choice as `vouchers.rs` (2.3) rather than the
//! legacy's "one grid, delete-and-reinsert every line on save" model (§4.2.2 steps 7-8) — this
//! document type has a real draft/posted/frozen lifecycle to edit incrementally against, unlike
//! `deposit_slips.rs`/`petty_cash.rs` (4.4), which chose whole-document writes precisely because
//! those documents have no such lifecycle. Header totals (`gross_amount`/`discount_amount`/
//! `tax_amount`/`total_amount`) are maintained incrementally in the same transaction as every line
//! mutation — never the legacy's out-of-transaction four-`UPDATE` recompute (§4.2.2 step 10).
//!
//! **Scope boundary, not a gap**: `status` only ever becomes `draft` from any handler in this
//! module. `posted`/`frozen` and `posted_voucher_id` are declared in the schema (0016 migration)
//! but wired by 5.8 (posting through the Phase 2.5 engine) and 5.7 (settlement, which blocks
//! delete). The manual test's own step 3 says as much: "Post it (once linked to accounting in
//! 5.8)". Production/transfer document types are 5.8's own Build bullet, not reached into here.
//!
//! **Judgment call, permission wiring**: unlike 2.3's vouchers (wired fully in the *same* phase,
//! by 2.8), the catalogue ids for invoice create/amend/delete (1404-1408, 1414, 08-04-authorization
//! .md) are wired *in this step itself* — the roadmap's own Build bullet asks for it explicitly
//! ("enforced server-side ... not only by disabling buttons like the legacy"), unlike Phase 3/4/
//! 5.1's deferred-to-a-later-step pattern. list/get have no catalogue id at all (the legacy list
//! screen gates action buttons, not the screen itself) — plain `AuthUser`, documented gap like
//! every other "no id exists" case in this codebase.

use crate::{audit, auth::AuthUser, db, AppState};
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

/// 08-04-authorization.md's `sRollOutPanel4` invoice-lifecycle ids.
pub(crate) mod permcodes {
    pub const CREATE_RECEIPT: &str = "issue_purchase_invoice"; // 1404
    pub const CREATE_ISSUE: &str = "issue_sales_invoice"; // 1405
    pub const CREATE_SALES_RETURN: &str = "sales_return_invoice"; // 1406
    pub const CREATE_PURCHASE_RETURN: &str = "purchase_return_invoice"; // 1407
    pub const AMEND: &str = "amend_invoice"; // 1408 — pistachio.rs reuses this for its own line-add
    pub const DELETE: &str = "delete_invoice"; // 1414

    pub fn create_code(document_type: &str) -> &'static str {
        match document_type {
            "receipt" => CREATE_RECEIPT,
            "issue" => CREATE_ISSUE,
            "sales_return" => CREATE_SALES_RETURN,
            "purchase_return" => CREATE_PURCHASE_RETURN,
            // 5.8: production/transfer have no legacy precedent at all (§3.2.4's "Not implemented
            // yet."), so no catalogue id exists for them either — AMEND (1408) is the closest fit,
            // same judgment call as the /post action's own permission choice.
            "production" | "transfer" => AMEND,
            _ => unreachable!("document_type is always one of the six enum values"),
        }
    }
}

pub(crate) fn require_permission(
    auth: &AuthUser,
    code: &str,
) -> Result<(), (StatusCode, Json<Value>)> {
    if auth.has_permission(code) {
        Ok(())
    } else {
        Err(forbidden("insufficient_permission"))
    }
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
fn forbidden(error: &str) -> (StatusCode, Json<Value>) {
    (StatusCode::FORBIDDEN, Json(json!({ "error": error })))
}
fn conflict_or_internal(err: sqlx::Error) -> (StatusCode, Json<Value>) {
    if let sqlx::Error::Database(db_err) = &err {
        if db_err.constraint() == Some("inventory_documents_number_key") {
            return (
                StatusCode::CONFLICT,
                Json(json!({ "error": "duplicate_document_number" })),
            );
        }
    }
    internal_error()
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(list_documents).post(create_document))
        .route(
            "/{id}",
            get(get_document)
                .put(update_document)
                .delete(delete_document),
        )
        .route("/{id}/lines", post(add_line))
        .route(
            "/{id}/lines/{line_id}",
            axum::routing::put(update_line).delete(delete_line),
        )
        .route(
            "/{id}/lines/pistachio",
            post(crate::pistachio::create_pistachio_line),
        )
        .route("/{id}/settlement", get(crate::settlement::get_settlement))
        .route(
            "/{id}/post",
            post(crate::inventory_posting::post_document_handler),
        )
}

#[derive(sqlx::FromRow, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DocumentRecord {
    pub(crate) id: i64,
    pub(crate) fiscal_year_id: i64,
    pub(crate) document_type: String,
    pub(crate) status: String,
    pub(crate) document_number: i32,
    pub(crate) document_date: NaiveDate,
    pub(crate) warehouse_id: i64,
    /// Nullable since 5.8 (production/transfer have no external counterparty) — still required
    /// for the four commercial types, enforced both by the DB `CHECK` (0018) and in Rust (B7).
    pub(crate) counterparty_account_id: Option<i64>,
    description: Option<String>,
    pub(crate) gross_amount: i64,
    pub(crate) discount_amount: i64,
    pub(crate) tax_amount: i64,
    pub(crate) total_amount: i64,
    pub(crate) posted_voucher_id: Option<i64>,
    /// 5.8: transfer's second warehouse leg. `None` for every other document type.
    pub(crate) destination_warehouse_id: Option<i64>,
}

const DOCUMENT_COLUMNS: &str = "id, fiscal_year_id, document_type::text, status::text, \
    document_number, document_date, warehouse_id, counterparty_account_id, description, \
    gross_amount, discount_amount, tax_amount, total_amount, posted_voucher_id, destination_warehouse_id";

#[derive(sqlx::FromRow, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct LineRecord {
    id: i64,
    document_id: i64,
    item_id: i64,
    quantity: bigdecimal::BigDecimal,
    unit_price: i64,
    gross_amount: i64,
    discount_amount: i64,
    tax_amount: i64,
    total_amount: i64,
    description: Option<String>,
}

const LINE_COLUMNS: &str = "id, document_id, item_id, quantity, unit_price, gross_amount, \
    discount_amount, tax_amount, total_amount, description";

pub(crate) async fn fetch_document(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: i64,
    id: i64,
) -> Result<Option<DocumentRecord>, sqlx::Error> {
    sqlx::query_as(&format!(
        "SELECT {DOCUMENT_COLUMNS} FROM inventory_documents WHERE tenant_id = $1 AND id = $2"
    ))
    .bind(tenant_id)
    .bind(id)
    .fetch_optional(&mut **tx)
    .await
}

async fn fetch_line(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: i64,
    document_id: i64,
    line_id: i64,
) -> Result<Option<LineRecord>, sqlx::Error> {
    sqlx::query_as(&format!(
        "SELECT {LINE_COLUMNS} FROM inventory_document_lines \
         WHERE tenant_id = $1 AND document_id = $2 AND id = $3"
    ))
    .bind(tenant_id)
    .bind(document_id)
    .bind(line_id)
    .fetch_optional(&mut **tx)
    .await
}

/// Same "is the fiscal year open" gate every other mutating module in this codebase applies
/// (03-03-c.md's `Is_New_Sanad_Valid`, ported uniformly per 2.3's own precedent) — the legacy
/// applies it to every mutating action in this module too (05-04-a.md §4.2.1's own citation list).
async fn fiscal_year_gate(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: i64,
    fiscal_year_id: i64,
) -> Result<Option<(bool, NaiveDate, NaiveDate)>, sqlx::Error> {
    sqlx::query_as(
        "SELECT is_active, start_date, end_date FROM fiscal_years WHERE tenant_id = $1 AND id = $2",
    )
    .bind(tenant_id)
    .bind(fiscal_year_id)
    .fetch_optional(&mut **tx)
    .await
}

/// B7 fix: the counterparty must exist and resolve to a real leaf account — no zero-sentinel path
/// through this validator at all, closing the legacy's unreachable `not S_Bed.tag=0` precedence
/// bug structurally rather than porting a fixed version of the same check.
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
        None => Err(bad_request("counterparty_account_not_found")),
        Some(c) if c > 0 => Err(bad_request("counterparty_account_not_leaf")),
        _ => Ok(()),
    }
}

// ---- list / get ------------------------------------------------------

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ListQuery {
    fiscal_year_id: Option<i64>,
    document_type: Option<String>,
    status: Option<String>,
    warehouse_id: Option<i64>,
}

async fn list_documents(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(q): Query<ListQuery>,
) -> Result<Json<Vec<DocumentRecord>>, (StatusCode, Json<Value>)> {
    let mut tx = db::begin(&state.pool, auth.tenant_id)
        .await
        .map_err(|_| internal_error())?;
    let rows = sqlx::query_as(&format!(
        "SELECT {DOCUMENT_COLUMNS} FROM inventory_documents WHERE tenant_id = $1 \
         AND ($2::bigint IS NULL OR fiscal_year_id = $2) \
         AND ($3::text IS NULL OR document_type::text = $3) \
         AND ($4::text IS NULL OR status::text = $4) \
         AND ($5::bigint IS NULL OR warehouse_id = $5) \
         ORDER BY document_date DESC, document_number DESC"
    ))
    .bind(auth.tenant_id)
    .bind(q.fiscal_year_id)
    .bind(&q.document_type)
    .bind(&q.status)
    .bind(q.warehouse_id)
    .fetch_all(&mut *tx)
    .await
    .map_err(|_| internal_error())?;
    tx.rollback().await.ok();
    Ok(Json(rows))
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct DocumentDetail {
    #[serde(flatten)]
    document: DocumentRecord,
    lines: Vec<LineRecord>,
}

async fn get_document(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<Json<DocumentDetail>, (StatusCode, Json<Value>)> {
    let mut tx = db::begin(&state.pool, auth.tenant_id)
        .await
        .map_err(|_| internal_error())?;
    let Some(document) = fetch_document(&mut tx, auth.tenant_id, id)
        .await
        .map_err(|_| internal_error())?
    else {
        return Err(not_found("document"));
    };
    let lines: Vec<LineRecord> = sqlx::query_as(&format!(
        "SELECT {LINE_COLUMNS} FROM inventory_document_lines WHERE tenant_id = $1 AND document_id = $2 ORDER BY id"
    ))
    .bind(auth.tenant_id)
    .bind(id)
    .fetch_all(&mut *tx)
    .await
    .map_err(|_| internal_error())?;
    tx.rollback().await.ok();
    Ok(Json(DocumentDetail { document, lines }))
}

// ---- create / update header -------------------------------------------

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateDocumentRequest {
    fiscal_year_id: i64,
    document_type: String,
    document_date: NaiveDate,
    warehouse_id: i64,
    /// Required for the four commercial types, forbidden-or-ignored for production/transfer (B7,
    /// still enforced — see `DocumentRecord::counterparty_account_id`'s doc comment).
    #[serde(default)]
    counterparty_account_id: Option<i64>,
    #[serde(default)]
    description: Option<String>,
    /// Omit to auto-allocate the next number in the fiscal year's shared sequence (§4.2.2 step 5
    /// — one sequence across all four types, not per-type).
    #[serde(default)]
    document_number: Option<i32>,
    /// 5.8, `transfer` only: the receiving warehouse.
    #[serde(default)]
    destination_warehouse_id: Option<i64>,
}

const VALID_TYPES: [&str; 6] = [
    "receipt",
    "issue",
    "purchase_return",
    "sales_return",
    "production",
    "transfer",
];
const COMMERCIAL_TYPES: [&str; 4] = ["receipt", "issue", "purchase_return", "sales_return"];

/// §4.2.1's create preconditions: fiscal year open (checked below via `fiscal_year_gate`),
/// per-type permission (`permcodes::create_code`), plus the B7 counterparty fix and §4.2.2 step 3
/// ("the invoice is empty" is enforced at `add_line` time in this incremental model — an empty
/// draft is allowed to exist transiently, exactly like `vouchers.rs`'s own "at least one line"
/// check being deferred to the draft->confirmed transition rather than blocking creation).
async fn create_document(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<CreateDocumentRequest>,
) -> Result<(StatusCode, Json<Value>), (StatusCode, Json<Value>)> {
    if !VALID_TYPES.contains(&req.document_type.as_str()) {
        return Err(bad_request("invalid_document_type"));
    }
    require_permission(&auth, permcodes::create_code(&req.document_type))?;

    let mut tx = db::begin(&state.pool, auth.tenant_id)
        .await
        .map_err(|_| internal_error())?;

    let Some((is_active, start_date, end_date)) =
        fiscal_year_gate(&mut tx, auth.tenant_id, req.fiscal_year_id)
            .await
            .map_err(|_| internal_error())?
    else {
        return Err(not_found("fiscal_year"));
    };
    if !is_active {
        return Err(forbidden("fiscal_year_closed"));
    }
    if req.document_date < start_date || req.document_date > end_date {
        return Err(bad_request("date_outside_fiscal_year"));
    }

    // B7, still enforced for the four commercial types (0018's own CHECK backs this up at the DB
    // layer too); production/transfer have no counterparty at all (5.8).
    if COMMERCIAL_TYPES.contains(&req.document_type.as_str()) {
        let Some(counterparty_id) = req.counterparty_account_id else {
            return Err(bad_request("counterparty_required"));
        };
        require_leaf_account(&mut tx, auth.tenant_id, counterparty_id).await?;
    } else if req.counterparty_account_id.is_some() {
        return Err(bad_request("counterparty_not_applicable")); // production/transfer: no external party
    }

    let warehouse_exists: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM warehouses WHERE tenant_id = $1 AND id = $2)",
    )
    .bind(auth.tenant_id)
    .bind(req.warehouse_id)
    .fetch_one(&mut *tx)
    .await
    .map_err(|_| internal_error())?;
    if !warehouse_exists {
        return Err(bad_request("warehouse_not_found"));
    }

    // 5.8, transfer only: a real second warehouse leg, distinct from the source.
    if req.document_type == "transfer" {
        let Some(destination_id) = req.destination_warehouse_id else {
            return Err(bad_request("destination_warehouse_required"));
        };
        if destination_id == req.warehouse_id {
            return Err(bad_request("destination_warehouse_must_differ"));
        }
        let destination_exists: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM warehouses WHERE tenant_id = $1 AND id = $2)",
        )
        .bind(auth.tenant_id)
        .bind(destination_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(|_| internal_error())?;
        if !destination_exists {
            return Err(bad_request("destination_warehouse_not_found"));
        }
    } else if req.destination_warehouse_id.is_some() {
        return Err(bad_request("destination_warehouse_not_applicable"));
    }

    let document_number = match req.document_number {
        Some(n) if n > 0 => n,
        Some(_) => return Err(bad_request("invalid_document_number")),
        None => {
            // Atomic per-fiscal-year allocator shared across all four types (§4.2.2 step 5),
            // same pattern as 2.3's voucher numbering — not the legacy's racy `SELECT MAX+1`.
            sqlx::query_scalar(
                "UPDATE fiscal_years SET next_inventory_document_number = \
                 next_inventory_document_number + 1 WHERE id = $1 \
                 RETURNING next_inventory_document_number - 1",
            )
            .bind(req.fiscal_year_id)
            .fetch_one(&mut *tx)
            .await
            .map_err(|_| internal_error())?
        }
    };

    let id: i64 = sqlx::query_scalar(
        "INSERT INTO inventory_documents \
         (tenant_id, fiscal_year_id, document_type, document_number, document_date, warehouse_id, \
          counterparty_account_id, description, destination_warehouse_id, created_by) \
         VALUES ($1, $2, $3::inventory_document_type, $4, $5, $6, $7, $8, $9, $10) RETURNING id",
    )
    .bind(auth.tenant_id)
    .bind(req.fiscal_year_id)
    .bind(&req.document_type)
    .bind(document_number)
    .bind(req.document_date)
    .bind(req.warehouse_id)
    .bind(req.counterparty_account_id)
    .bind(req.description.as_deref().map(str::trim))
    .bind(req.destination_warehouse_id)
    .bind(auth.user_id)
    .fetch_one(&mut *tx)
    .await
    .map_err(conflict_or_internal)?;

    audit::record_mutation(
        &mut tx,
        auth.tenant_id,
        "inventory_documents",
        id,
        "insert",
        Some(auth.user_id),
        None,
        Some(json!({ "documentType": req.document_type, "documentNumber": document_number })),
    )
    .await
    .map_err(|_| internal_error())?;

    tx.commit().await.map_err(|_| internal_error())?;
    Ok((StatusCode::CREATED, Json(json!({ "id": id }))))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct UpdateDocumentRequest {
    document_date: NaiveDate,
    warehouse_id: i64,
    #[serde(default)]
    counterparty_account_id: Option<i64>,
    #[serde(default)]
    description: Option<String>,
}

/// §4.4's "mutable while draft" table, extended by 5.8: header fields are editable while `status`
/// is `draft` **or** `posted` — 5.2's Build bullet already said posted documents stay editable
/// "per the legacy's actual model"; only `frozen` (the voucher itself finalised, §4.1) is
/// read-only. Editing a posted document does *not* auto-repost it — call `/post` again for that
/// (manual test #4's own "edit and save again" framing), so the linked voucher can go stale
/// between an edit and the next explicit post, exactly like the legacy's own "settled invoice can
/// still be edited" gap (§9.5) except here there's a real re-post action to close it, not silence.
/// `document_type` and `document_number` are deliberately absent from this request: the type is
/// fixed at creation (no legacy renumber-the-type concept exists) and the number is left to a
/// dedicated action if ever needed (not in this step's scope, matching 2.6's own extension point
/// for vouchers being a separate, later addition rather than folded in here speculatively).
async fn update_document(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
    Json(req): Json<UpdateDocumentRequest>,
) -> Result<StatusCode, (StatusCode, Json<Value>)> {
    require_permission(&auth, permcodes::AMEND)?;
    let mut tx = db::begin(&state.pool, auth.tenant_id)
        .await
        .map_err(|_| internal_error())?;
    let Some(document) = fetch_document(&mut tx, auth.tenant_id, id)
        .await
        .map_err(|_| internal_error())?
    else {
        return Err(not_found("document"));
    };
    if document.status == "frozen" {
        return Err(bad_request("document_frozen"));
    }

    let Some((is_active, start_date, end_date)) =
        fiscal_year_gate(&mut tx, auth.tenant_id, document.fiscal_year_id)
            .await
            .map_err(|_| internal_error())?
    else {
        return Err(internal_error());
    };
    if !is_active {
        return Err(forbidden("fiscal_year_closed"));
    }
    if req.document_date < start_date || req.document_date > end_date {
        return Err(bad_request("date_outside_fiscal_year"));
    }
    if COMMERCIAL_TYPES.contains(&document.document_type.as_str()) {
        let Some(counterparty_id) = req.counterparty_account_id else {
            return Err(bad_request("counterparty_required"));
        };
        require_leaf_account(&mut tx, auth.tenant_id, counterparty_id).await?; // B7, re-checked on every edit
    } else if req.counterparty_account_id.is_some() {
        return Err(bad_request("counterparty_not_applicable"));
    }

    let warehouse_exists: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM warehouses WHERE tenant_id = $1 AND id = $2)",
    )
    .bind(auth.tenant_id)
    .bind(req.warehouse_id)
    .fetch_one(&mut *tx)
    .await
    .map_err(|_| internal_error())?;
    if !warehouse_exists {
        return Err(bad_request("warehouse_not_found"));
    }

    sqlx::query(
        "UPDATE inventory_documents SET document_date = $1, warehouse_id = $2, \
         counterparty_account_id = $3, description = $4, updated_at = now(), updated_by = $5 \
         WHERE id = $6",
    )
    .bind(req.document_date)
    .bind(req.warehouse_id)
    .bind(req.counterparty_account_id)
    .bind(req.description.as_deref().map(str::trim))
    .bind(auth.user_id)
    .bind(id)
    .execute(&mut *tx)
    .await
    .map_err(|_| internal_error())?;

    audit::record_mutation(
        &mut tx,
        auth.tenant_id,
        "inventory_documents",
        id,
        "update",
        Some(auth.user_id),
        Some(json!({ "counterpartyAccountId": document.counterparty_account_id })),
        Some(json!({ "counterpartyAccountId": req.counterparty_account_id })),
    )
    .await
    .map_err(|_| internal_error())?;

    tx.commit().await.map_err(|_| internal_error())?;
    Ok(StatusCode::NO_CONTENT)
}

/// §4.2.5's delete preconditions #3-6 (voucher exists/in draft, no deposit slip, no cheque) are
/// all downstream of 5.7/5.8 machinery that doesn't exist yet — `status = 'draft'` is the only
/// guard this step can enforce, and it's also the only state any handler here can ever produce.
/// B8/B9 (5.8): "un-post (delete) a posted document" is one action here — deleting a `posted`
/// document deletes its linked voucher first (cascading every voucher line via `ON DELETE
/// CASCADE`, 0006's schema), then the document's own lines and header. No enumerated `M_Id`
/// subset to fall out of sync with a new posting rule (§5.3's B8/B9 finding) — the FK is the only
/// list. `frozen` (the voucher itself finalised elsewhere) is the only status this refuses.
async fn delete_document(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<StatusCode, (StatusCode, Json<Value>)> {
    require_permission(&auth, permcodes::DELETE)?;
    let mut tx = db::begin(&state.pool, auth.tenant_id)
        .await
        .map_err(|_| internal_error())?;
    let Some(document) = fetch_document(&mut tx, auth.tenant_id, id)
        .await
        .map_err(|_| internal_error())?
    else {
        return Err(not_found("document"));
    };
    if document.status == "frozen" {
        return Err(bad_request("document_frozen"));
    }

    // The document row's own posted_voucher_id FK must be gone before the voucher can be deleted
    // (no ON DELETE action declared on that column, 0016's own migration — deliberately, since
    // this is the only path that ever removes a posted voucher, unlike voucher_lines' CASCADE).
    sqlx::query("DELETE FROM inventory_document_lines WHERE tenant_id = $1 AND document_id = $2")
        .bind(auth.tenant_id)
        .bind(id)
        .execute(&mut *tx)
        .await
        .map_err(|_| internal_error())?;
    sqlx::query("DELETE FROM inventory_documents WHERE id = $1")
        .bind(id)
        .execute(&mut *tx)
        .await
        .map_err(|_| internal_error())?;
    if let Some(voucher_id) = document.posted_voucher_id {
        sqlx::query("DELETE FROM vouchers WHERE tenant_id = $1 AND id = $2")
            .bind(auth.tenant_id)
            .bind(voucher_id)
            .execute(&mut *tx)
            .await
            .map_err(|_| internal_error())?;
    }

    audit::record_mutation(
        &mut tx, auth.tenant_id, "inventory_documents", id, "delete", Some(auth.user_id),
        Some(json!({ "documentType": document.document_type, "documentNumber": document.document_number })),
        None,
    )
    .await
    .map_err(|_| internal_error())?;

    tx.commit().await.map_err(|_| internal_error())?;
    Ok(StatusCode::NO_CONTENT)
}

// ---- lines -------------------------------------------------------------

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct LineRequest {
    item_id: i64,
    quantity: bigdecimal::BigDecimal,
    /// §7.1: the item's `sale_price` is only ever a *default* the caller applies before sending
    /// this request (client-side, per this step's own manual test #2) — this field has no
    /// server-side fallback to it at all, so "silently reset to the list price" cannot happen
    /// here structurally, the same reasoning 5.4 already established for the average-cost default.
    unit_price: i64,
    /// Path 1 (§7.2): an absolute discount amount.
    #[serde(default)]
    discount_amount: i64,
    /// Path 2 (§7.2): a percentage of the line's gross amount — mutually exclusive with
    /// `discount_amount` (the legacy's two boxes both write the same `AFD_Kasr` column; this API
    /// makes the ambiguity a rejected request instead of "whichever control fired last wins").
    #[serde(default)]
    discount_percent: Option<bigdecimal::BigDecimal>,
    #[serde(default)]
    tax_amount: i64,
    #[serde(default)]
    description: Option<String>,
}

pub(crate) struct LineAmounts {
    pub(crate) gross: i64,
    pub(crate) discount: i64,
    pub(crate) tax: i64,
    pub(crate) total: i64,
}

/// §3.1.2's gross formula, correctly rounded (not the legacy's `trunc`, per 05-06-a.md §6.1's
/// downward-bias finding — that finding is about costing specifically, but the same fix applies
/// here since nothing in this step's spec asks for the legacy's truncation to be preserved),
/// §7.2's dual-mode discount (`discount_amount = round(gross × discount_pct / 100)` when entered
/// as a percentage — correctly rounded, not the legacy's `trunc`, per this step's own Build
/// bullet), and §4.2.2 step 10's net formula: `total = gross + tax - discount`.
fn compute_line_amounts(
    quantity: &bigdecimal::BigDecimal,
    unit_price: i64,
    discount_amount: i64,
    discount_percent: Option<&bigdecimal::BigDecimal>,
    tax: i64,
) -> LineAmounts {
    let gross_decimal = quantity * bigdecimal::BigDecimal::from(unit_price);
    let gross = crate::money::round_to_rial(&gross_decimal);
    let discount = match discount_percent {
        Some(pct) => crate::money::round_to_rial(
            &(bigdecimal::BigDecimal::from(gross) * pct / bigdecimal::BigDecimal::from(100)),
        ),
        None => discount_amount,
    };
    let total = gross + tax - discount;
    LineAmounts {
        gross,
        discount,
        tax,
        total,
    }
}

async fn validate_line_request(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: i64,
    req: &LineRequest,
) -> Result<(), (StatusCode, Json<Value>)> {
    use bigdecimal::Zero;
    if req.quantity <= bigdecimal::BigDecimal::zero() {
        return Err(bad_request("invalid_quantity"));
    }
    if req.discount_amount < 0 || req.tax_amount < 0 {
        return Err(bad_request("invalid_amount"));
    }
    if let Some(pct) = &req.discount_percent {
        if req.discount_amount != 0 {
            return Err(bad_request("ambiguous_discount_entry")); // §7.2: pick one entry mode, not both
        }
        #[allow(clippy::cmp_owned)]
        // comparing an operator-entered percentage against the 0..=100 range
        let out_of_range = pct.sign() == bigdecimal::num_bigint::Sign::Minus
            || *pct > bigdecimal::BigDecimal::from(100);
        if out_of_range {
            return Err(bad_request("invalid_discount_percent")); // §7.2: KasrD's own 0.00-99.99 range
        }
    }
    let item_exists: bool =
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM items WHERE tenant_id = $1 AND id = $2)")
            .bind(tenant_id)
            .bind(req.item_id)
            .fetch_one(&mut **tx)
            .await
            .map_err(|_| internal_error())?;
    if !item_exists {
        return Err(bad_request("item_not_found"));
    }
    Ok(())
}

/// The shared "insert a line, bump the header's incrementally-maintained totals" pair — the same
/// two statements `add_line` always ran, extracted so 5.6's pistachio calculator (which computes
/// `quantity`/`unit_price` from a deduction formula rather than taking them as direct client
/// input) can reuse the identical insert-and-total-maintenance path instead of duplicating it.
/// Does **not** commit or audit — the caller owns the transaction and writes its own audit row,
/// since the two callers describe what happened differently in the log.
pub(crate) async fn insert_line_and_bump_totals(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: i64,
    document: &DocumentRecord,
    item_id: i64,
    quantity: &bigdecimal::BigDecimal,
    unit_price: i64,
    amounts: &LineAmounts,
    description: Option<&str>,
    user_id: i64,
) -> Result<i64, sqlx::Error> {
    let line_id: i64 = sqlx::query_scalar(
        "INSERT INTO inventory_document_lines \
         (tenant_id, fiscal_year_id, document_id, item_id, quantity, unit_price, gross_amount, \
          discount_amount, tax_amount, total_amount, description, created_by) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12) RETURNING id",
    )
    .bind(tenant_id)
    .bind(document.fiscal_year_id)
    .bind(document.id)
    .bind(item_id)
    .bind(quantity)
    .bind(unit_price)
    .bind(amounts.gross)
    .bind(amounts.discount)
    .bind(amounts.tax)
    .bind(amounts.total)
    .bind(description)
    .bind(user_id)
    .fetch_one(&mut **tx)
    .await?;

    sqlx::query(
        "UPDATE inventory_documents SET gross_amount = gross_amount + $1, \
         discount_amount = discount_amount + $2, tax_amount = tax_amount + $3, \
         total_amount = total_amount + $4, updated_at = now(), updated_by = $5 WHERE id = $6",
    )
    .bind(amounts.gross)
    .bind(amounts.discount)
    .bind(amounts.tax)
    .bind(amounts.total)
    .bind(user_id)
    .bind(document.id)
    .execute(&mut **tx)
    .await?;

    Ok(line_id)
}

async fn add_line(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
    Json(req): Json<LineRequest>,
) -> Result<(StatusCode, Json<Value>), (StatusCode, Json<Value>)> {
    require_permission(&auth, permcodes::AMEND)?;
    let mut tx = db::begin(&state.pool, auth.tenant_id)
        .await
        .map_err(|_| internal_error())?;
    let Some(document) = fetch_document(&mut tx, auth.tenant_id, id)
        .await
        .map_err(|_| internal_error())?
    else {
        return Err(not_found("document"));
    };
    if document.status == "frozen" {
        return Err(bad_request("document_frozen"));
    }
    validate_line_request(&mut tx, auth.tenant_id, &req).await?;
    let amounts = compute_line_amounts(
        &req.quantity,
        req.unit_price,
        req.discount_amount,
        req.discount_percent.as_ref(),
        req.tax_amount,
    );

    let line_id = insert_line_and_bump_totals(
        &mut tx,
        auth.tenant_id,
        &document,
        req.item_id,
        &req.quantity,
        req.unit_price,
        &amounts,
        req.description.as_deref().map(str::trim),
        auth.user_id,
    )
    .await
    .map_err(|_| internal_error())?;

    audit::record_mutation(
        &mut tx, auth.tenant_id, "inventory_document_lines", line_id, "insert", Some(auth.user_id), None,
        Some(json!({ "documentId": id, "itemId": req.item_id, "quantity": req.quantity.to_string(), "totalAmount": amounts.total })),
    )
    .await
    .map_err(|_| internal_error())?;

    tx.commit().await.map_err(|_| internal_error())?;
    Ok((StatusCode::CREATED, Json(json!({ "id": line_id }))))
}

async fn update_line(
    State(state): State<AppState>,
    auth: AuthUser,
    Path((id, line_id)): Path<(i64, i64)>,
    Json(req): Json<LineRequest>,
) -> Result<StatusCode, (StatusCode, Json<Value>)> {
    require_permission(&auth, permcodes::AMEND)?;
    let mut tx = db::begin(&state.pool, auth.tenant_id)
        .await
        .map_err(|_| internal_error())?;
    let Some(document) = fetch_document(&mut tx, auth.tenant_id, id)
        .await
        .map_err(|_| internal_error())?
    else {
        return Err(not_found("document"));
    };
    if document.status == "frozen" {
        return Err(bad_request("document_frozen"));
    }
    let Some(line) = fetch_line(&mut tx, auth.tenant_id, id, line_id)
        .await
        .map_err(|_| internal_error())?
    else {
        return Err(not_found("line"));
    };
    validate_line_request(&mut tx, auth.tenant_id, &req).await?;
    let amounts = compute_line_amounts(
        &req.quantity,
        req.unit_price,
        req.discount_amount,
        req.discount_percent.as_ref(),
        req.tax_amount,
    );

    sqlx::query(
        "UPDATE inventory_document_lines SET item_id = $1, quantity = $2, unit_price = $3, \
         gross_amount = $4, discount_amount = $5, tax_amount = $6, total_amount = $7, \
         description = $8, updated_at = now(), updated_by = $9 WHERE id = $10",
    )
    .bind(req.item_id)
    .bind(&req.quantity)
    .bind(req.unit_price)
    .bind(amounts.gross)
    .bind(amounts.discount)
    .bind(req.tax_amount)
    .bind(amounts.total)
    .bind(req.description.as_deref().map(str::trim))
    .bind(auth.user_id)
    .bind(line_id)
    .execute(&mut *tx)
    .await
    .map_err(|_| internal_error())?;

    sqlx::query(
        "UPDATE inventory_documents SET gross_amount = gross_amount - $1 + $2, \
         discount_amount = discount_amount - $3 + $4, tax_amount = tax_amount - $5 + $6, \
         total_amount = total_amount - $7 + $8, updated_at = now(), updated_by = $9 WHERE id = $10",
    )
    .bind(line.gross_amount)
    .bind(amounts.gross)
    .bind(line.discount_amount)
    .bind(amounts.discount)
    .bind(line.tax_amount)
    .bind(req.tax_amount)
    .bind(line.total_amount)
    .bind(amounts.total)
    .bind(auth.user_id)
    .bind(id)
    .execute(&mut *tx)
    .await
    .map_err(|_| internal_error())?;

    audit::record_mutation(
        &mut tx,
        auth.tenant_id,
        "inventory_document_lines",
        line_id,
        "update",
        Some(auth.user_id),
        Some(json!({ "totalAmount": line.total_amount })),
        Some(json!({ "totalAmount": amounts.total })),
    )
    .await
    .map_err(|_| internal_error())?;

    tx.commit().await.map_err(|_| internal_error())?;
    Ok(StatusCode::NO_CONTENT)
}

async fn delete_line(
    State(state): State<AppState>,
    auth: AuthUser,
    Path((id, line_id)): Path<(i64, i64)>,
) -> Result<StatusCode, (StatusCode, Json<Value>)> {
    require_permission(&auth, permcodes::AMEND)?;
    let mut tx = db::begin(&state.pool, auth.tenant_id)
        .await
        .map_err(|_| internal_error())?;
    let Some(document) = fetch_document(&mut tx, auth.tenant_id, id)
        .await
        .map_err(|_| internal_error())?
    else {
        return Err(not_found("document"));
    };
    if document.status == "frozen" {
        return Err(bad_request("document_frozen"));
    }
    let Some(line) = fetch_line(&mut tx, auth.tenant_id, id, line_id)
        .await
        .map_err(|_| internal_error())?
    else {
        return Err(not_found("line"));
    };

    sqlx::query("DELETE FROM inventory_document_lines WHERE id = $1")
        .bind(line_id)
        .execute(&mut *tx)
        .await
        .map_err(|_| internal_error())?;

    sqlx::query(
        "UPDATE inventory_documents SET gross_amount = gross_amount - $1, \
         discount_amount = discount_amount - $2, tax_amount = tax_amount - $3, \
         total_amount = total_amount - $4, updated_at = now(), updated_by = $5 WHERE id = $6",
    )
    .bind(line.gross_amount)
    .bind(line.discount_amount)
    .bind(line.tax_amount)
    .bind(line.total_amount)
    .bind(auth.user_id)
    .bind(id)
    .execute(&mut *tx)
    .await
    .map_err(|_| internal_error())?;

    audit::record_mutation(
        &mut tx,
        auth.tenant_id,
        "inventory_document_lines",
        line_id,
        "delete",
        Some(auth.user_id),
        Some(json!({ "totalAmount": line.total_amount })),
        None,
    )
    .await
    .map_err(|_| internal_error())?;

    tx.commit().await.map_err(|_| internal_error())?;
    Ok(StatusCode::NO_CONTENT)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn line_amounts_round_correctly_not_truncate() {
        // 3 * 333.333 = 999.999 -> rounds to 1000, the legacy's trunc() would give 999.
        let qty = "3.333".parse::<bigdecimal::BigDecimal>().unwrap();
        let amounts = compute_line_amounts(&qty, 300, 0, None, 0);
        assert_eq!(amounts.gross, 1000);
        assert_eq!(amounts.total, 1000);
    }

    #[test]
    fn total_is_gross_plus_tax_minus_discount() {
        let qty = bigdecimal::BigDecimal::from(10);
        let amounts = compute_line_amounts(&qty, 1000, 500, None, 200);
        assert_eq!(amounts.gross, 10000);
        assert_eq!(amounts.discount, 500);
        assert_eq!(amounts.total, 10000 + 200 - 500);
    }

    /// Manual test #1: a percentage discount is computed correctly rounded, not truncated.
    #[test]
    fn discount_percent_rounds_correctly_not_truncate() {
        // gross = 10 * 1000 = 10,000; 3.335% of 10,000 = 333.5 -> rounds to 334, trunc would give 333.
        let qty = bigdecimal::BigDecimal::from(10);
        let pct = "3.335".parse::<bigdecimal::BigDecimal>().unwrap();
        let amounts = compute_line_amounts(&qty, 1000, 0, Some(&pct), 0);
        assert_eq!(amounts.gross, 10000);
        assert_eq!(amounts.discount, 334);
        assert_eq!(amounts.total, 10000 - 334);
    }
}
