//! Step 5.8 (docs/phase-5-inventory.md §5.8): every inventory document type posts a correctly
//! balanced voucher through the Phase 2.5 engine (`auto_post::post_generated_voucher`, which
//! rejects an unbalanced posting outright — the structural fix for B1). No verified legacy
//! posting rule was recoverable (`Anbar_AddToFactor`'s body is unavailable, specs/05-inventory/
//! 05-10-a.md §10.1.1's "single largest unknown in the inventory domain"), so this module's rules
//! are the rebuild's own clean design, per the Build bullet's own framing — but the discount/VAT
//! placement is **not** a guess: it is exactly 05-10-a.md §10.2.5's own "the correct entry would
//! be" worked correction for `init12`'s `2·discount − VAT` imbalance, generalised symmetrically to
//! all four commercial document types (derivation in this module's own doc comments below).
//!
//! **B2 fix**: `production` and `transfer` are new document types with real, explicit, balanced
//! postings — no legacy precedent exists for either (§3.2.4: `' Not implemented yet. '` for both).
//! New warehouse account roles (`finished_goods_account_id`/`raw_materials_account_id` for
//! production, `inventory_account_id` on both legs for transfer's wash entry) since none of the
//! six existing commercial roles (5.1) mean anything for an internal stock movement.
//!
//! **B8/B9 fix**: un-posting is "delete the linked voucher" — one `DELETE FROM vouchers WHERE id =
//! <posted_voucher_id>`, cascading to every voucher line via `ON DELETE CASCADE` (0006's own
//! schema). No enumerated `M_Id` subset to fall out of sync with a new posting rule, because there
//! is no subset at all — the FK is the only list.
//!
//! **Idempotent re-post** (manual test #4): `post_document` is safe to call on an already-`posted`
//! document — it deletes the existing voucher (cascading its lines) and posts a fresh one in the
//! *same* transaction as the delete, so a mid-operation failure leaves the old voucher intact
//! (transactional atomicity), never a half-deleted, half-reposted state.

use crate::{
    audit,
    auto_post::{self, GeneratedLine, PostingError},
    db,
    inventory_documents::{self, DocumentRecord},
    AppState,
};
use axum::{extract::{Path, State}, http::StatusCode, Json};
use serde_json::{json, Value};
use sqlx::{Postgres, Transaction};

fn internal_error() -> (StatusCode, Json<Value>) {
    (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": "internal_error" })))
}
fn bad_request(error: &str) -> (StatusCode, Json<Value>) {
    (StatusCode::BAD_REQUEST, Json(json!({ "error": error })))
}
fn not_found(error: &str) -> (StatusCode, Json<Value>) {
    (StatusCode::NOT_FOUND, Json(json!({ "error": error })))
}

/// `journal_sources.id` — "inventory_invoice", seeded by 2.3, shared by every inventory document
/// type (they all ultimately come from the one `inventory_documents` table).
const SOURCE_KIND: i16 = 1;

#[derive(sqlx::FromRow)]
struct WarehouseAccounts {
    purchase_account_id: i64,
    purchase_return_account_id: i64,
    sales_account_id: i64,
    sales_return_account_id: i64,
    discount_account_id: i64,
    vat_account_id: i64,
    finished_goods_account_id: Option<i64>,
    raw_materials_account_id: Option<i64>,
    inventory_account_id: Option<i64>,
}

async fn fetch_warehouse_accounts(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: i64,
    warehouse_id: i64,
) -> Result<Option<WarehouseAccounts>, sqlx::Error> {
    sqlx::query_as(
        "SELECT purchase_account_id, purchase_return_account_id, sales_account_id, \
         sales_return_account_id, discount_account_id, vat_account_id, finished_goods_account_id, \
         raw_materials_account_id, inventory_account_id \
         FROM warehouses WHERE tenant_id = $1 AND id = $2",
    )
    .bind(tenant_id)
    .bind(warehouse_id)
    .fetch_optional(&mut **tx)
    .await
}

/// §10.2.5's own worked correction, generalised: for the two **inbound** types (receipt,
/// sales_return — both increase stock, §5.1.1's direction table), `Dr primary(gross) + Dr
/// vat(tax) = Cr counterparty(total) + Cr discount(discount)`. For the two **outbound** types
/// (issue, purchase_return), the whole entry mirrors: `Dr counterparty(total) + Dr
/// discount(discount) = Cr primary(gross) + Cr vat(tax)`. Verified balanced algebraically:
/// `total = gross + tax - discount`, so both sides always reduce to `gross + tax` regardless of
/// discount/tax being zero, negative-free, or absent — the engine's own balance check is the
/// final proof, not just this comment.
fn commercial_lines(
    document: &DocumentRecord,
    accounts: &WarehouseAccounts,
    narration: &str,
) -> Result<Vec<GeneratedLine>, (StatusCode, Json<Value>)> {
    let Some(counterparty) = document.counterparty_account_id else {
        return Err(bad_request("counterparty_required")); // B7, still enforced for these four types
    };
    let primary = match document.document_type.as_str() {
        "receipt" => accounts.purchase_account_id,
        "issue" => accounts.sales_account_id,
        "purchase_return" => accounts.purchase_return_account_id,
        "sales_return" => accounts.sales_return_account_id,
        other => return Err(internal_error_with(other)),
    };
    let inbound = matches!(document.document_type.as_str(), "receipt" | "sales_return");

    let mut lines = Vec::with_capacity(4);
    let d = |account_id, debit, credit| GeneratedLine { account_id, debit, credit, description: narration.to_string() };
    if inbound {
        lines.push(d(primary, document.gross_amount, 0));
        if document.tax_amount > 0 {
            lines.push(d(accounts.vat_account_id, document.tax_amount, 0));
        }
        lines.push(d(counterparty, 0, document.total_amount));
        if document.discount_amount > 0 {
            lines.push(d(accounts.discount_account_id, 0, document.discount_amount));
        }
    } else {
        lines.push(d(counterparty, document.total_amount, 0));
        if document.discount_amount > 0 {
            lines.push(d(accounts.discount_account_id, document.discount_amount, 0));
        }
        lines.push(d(primary, 0, document.gross_amount));
        if document.tax_amount > 0 {
            lines.push(d(accounts.vat_account_id, 0, document.tax_amount));
        }
    }
    Ok(lines)
}

fn internal_error_with(_unused: &str) -> (StatusCode, Json<Value>) {
    internal_error()
}

/// B2: production debits the warehouse's finished-goods account and credits its raw-materials
/// account for the document's own total — a genuinely new rule, no legacy precedent (module doc
/// comment).
fn production_lines(
    document: &DocumentRecord,
    accounts: &WarehouseAccounts,
    narration: &str,
) -> Result<Vec<GeneratedLine>, (StatusCode, Json<Value>)> {
    let Some(finished_goods) = accounts.finished_goods_account_id else {
        return Err(bad_request("finished_goods_account_not_configured"));
    };
    let Some(raw_materials) = accounts.raw_materials_account_id else {
        return Err(bad_request("raw_materials_account_not_configured"));
    };
    let amount = document.total_amount;
    Ok(vec![
        GeneratedLine { account_id: finished_goods, debit: amount, credit: 0, description: narration.to_string() },
        GeneratedLine { account_id: raw_materials, debit: 0, credit: amount, description: narration.to_string() },
    ])
}

/// B2: transfer is a wash entry between the source (`document.warehouse_id`) and destination
/// warehouses' `inventory_account_id` — net zero across the pair, per the Build bullet's own words.
fn transfer_lines(
    document: &DocumentRecord,
    source_accounts: &WarehouseAccounts,
    destination_accounts: &WarehouseAccounts,
    narration: &str,
) -> Result<Vec<GeneratedLine>, (StatusCode, Json<Value>)> {
    let Some(source_inventory) = source_accounts.inventory_account_id else {
        return Err(bad_request("source_inventory_account_not_configured"));
    };
    let Some(destination_inventory) = destination_accounts.inventory_account_id else {
        return Err(bad_request("destination_inventory_account_not_configured"));
    };
    let amount = document.total_amount;
    Ok(vec![
        GeneratedLine { account_id: destination_inventory, debit: amount, credit: 0, description: narration.to_string() },
        GeneratedLine { account_id: source_inventory, debit: 0, credit: amount, description: narration.to_string() },
    ])
}

/// B fix #6: an accurate, distinct narration per type — never the legacy's shared `'فروش کالا '`
/// ("goods sale") label for all four commercial types plus two more that never existed before.
fn narration_for(document: &DocumentRecord) -> String {
    let label = match document.document_type.as_str() {
        "receipt" => "Purchase",
        "issue" => "Sale",
        "purchase_return" => "Purchase return",
        "sales_return" => "Sales return",
        "production" => "Production",
        "transfer" => "Warehouse transfer",
        other => other,
    };
    format!("{label} — invoice #{}", document.document_number)
}

/// The one posting action. Idempotent: an already-`posted` document is re-posted (module doc
/// comment) rather than rejected — this *is* the "re-save" manual test #4 exercises.
pub(crate) async fn post_document(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: i64,
    document: &DocumentRecord,
    actor_id: i64,
) -> Result<i64, (StatusCode, Json<Value>)> {
    let narration = narration_for(document);

    let Some(warehouse_accounts) = fetch_warehouse_accounts(tx, tenant_id, document.warehouse_id)
        .await
        .map_err(|_| internal_error())?
    else {
        return Err(not_found("warehouse_not_found"));
    };

    let lines = match document.document_type.as_str() {
        "receipt" | "issue" | "purchase_return" | "sales_return" => {
            commercial_lines(document, &warehouse_accounts, &narration)?
        }
        "production" => production_lines(document, &warehouse_accounts, &narration)?,
        "transfer" => {
            let Some(destination_warehouse_id) = document.destination_warehouse_id else {
                return Err(bad_request("destination_warehouse_required"));
            };
            let Some(destination_accounts) = fetch_warehouse_accounts(tx, tenant_id, destination_warehouse_id)
                .await
                .map_err(|_| internal_error())?
            else {
                return Err(not_found("destination_warehouse_not_found"));
            };
            transfer_lines(document, &warehouse_accounts, &destination_accounts, &narration)?
        }
        other => return Err(internal_error_with(other)),
    };

    // Idempotent re-post (manual test #4): delete the stale voucher first, in the SAME
    // transaction as the fresh post -- a failure partway through leaves the old voucher intact.
    // The document's own posted_voucher_id FK must be cleared before the voucher row can go
    // (no ON DELETE action on that column, 0016's migration).
    if let Some(old_voucher_id) = document.posted_voucher_id {
        sqlx::query("UPDATE inventory_documents SET posted_voucher_id = NULL WHERE id = $1")
            .bind(document.id)
            .execute(&mut **tx)
            .await
            .map_err(|_| internal_error())?;
        sqlx::query("DELETE FROM vouchers WHERE tenant_id = $1 AND id = $2")
            .bind(tenant_id)
            .bind(old_voucher_id)
            .execute(&mut **tx)
            .await
            .map_err(|_| internal_error())?;
    }

    let voucher_id = auto_post::post_generated_voucher(
        tx, tenant_id, document.fiscal_year_id, document.document_date, &narration, SOURCE_KIND, document.id,
        &lines, actor_id,
    )
    .await
    .map_err(posting_error_response)?;

    sqlx::query(
        "UPDATE inventory_documents SET status = 'posted', posted_voucher_id = $1, updated_at = now(), \
         updated_by = $2 WHERE id = $3",
    )
    .bind(voucher_id)
    .bind(actor_id)
    .bind(document.id)
    .execute(&mut **tx)
    .await
    .map_err(|_| internal_error())?;

    Ok(voucher_id)
}

fn posting_error_response(err: PostingError) -> (StatusCode, Json<Value>) {
    match err {
        PostingError::AccountNotFound(_) => not_found("posting_account_not_found"),
        PostingError::AccountNotLeaf(_) => bad_request("posting_account_not_leaf"),
        PostingError::FiscalYearNotFound => not_found("fiscal_year"),
        PostingError::FiscalYearClosed => bad_request("fiscal_year_closed"),
        PostingError::EmptyLines | PostingError::Unbalanced | PostingError::InvalidLineAmount(_) | PostingError::Database(_) => {
            internal_error()
        }
    }
}

/// `POST /api/v1/inventory-documents/{id}/post` — 1408 (`amend_invoice`), the closest catalogue
/// fit (no legacy id exists for "post", since subsystem A posted inline on save; a judgment call,
/// documented here rather than at the call site).
pub(crate) async fn post_document_handler(
    State(state): State<AppState>,
    auth: crate::auth::AuthUser,
    Path(document_id): Path<i64>,
) -> Result<StatusCode, (StatusCode, Json<Value>)> {
    inventory_documents::require_permission(&auth, inventory_documents::permcodes::AMEND)?;
    let mut tx = db::begin(&state.pool, auth.tenant_id).await.map_err(|_| internal_error())?;
    let Some(document) = inventory_documents::fetch_document(&mut tx, auth.tenant_id, document_id)
        .await
        .map_err(|_| internal_error())?
    else {
        return Err(not_found("document_not_found"));
    };
    if document.status == "frozen" {
        return Err(bad_request("document_frozen"));
    }

    let voucher_id = post_document(&mut tx, auth.tenant_id, &document, auth.user_id).await?;

    audit::record_mutation(
        &mut tx, auth.tenant_id, "inventory_documents", document_id, "update",
        Some(auth.user_id),
        Some(json!({ "status": document.status })),
        Some(json!({ "status": "posted", "postedVoucherId": voucher_id })),
    )
    .await
    .map_err(|_| internal_error())?;

    tx.commit().await.map_err(|_| internal_error())?;
    Ok(StatusCode::NO_CONTENT)
}
