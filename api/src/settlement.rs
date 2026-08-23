//! Step 5.7 (docs/phase-5-inventory.md §5.7): settlement (Tasfieh) — attaching treasury
//! instruments (deposit slips, received cheques) to an invoice. specs/05-inventory/05-09-a/b.md
//! §9.0 is explicit: **there is no settlement algorithm** — no allocation, no FIFO matching, no
//! partial/full status. This module ports exactly that thin scope: a filtered, sorted list plus a
//! computed outstanding figure. Do not read more into it than the legacy actually had.
//!
//! **Linking mechanism**: Phase 4's `deposit_slips`/`received_cheques` already carry
//! `source_module`/`source_id` (legacy `S_LinkPRG`/`S_LinkSSN`), left unset by every 4.x handler
//! until a real caller needed them. This step is that caller — `received_cheques::ReceiveFields`
//! and `deposit_slips::SlipFields` both gained an optional `source_document_id`, set at creation
//! time (mirroring `New_From_PRg`'s pre-filled-link flow, §9.4.1), mapped to
//! `source_module = 1` ("inventory_invoice", already seeded by 2.3) and
//! `source_id = <inventory_documents.id>`. **A real surrogate-key link from day one** — §9.7 point
//! 1's own ask — never the legacy's mutable invoice-*number* link (§9.1's "definitive proof the
//! treasury link is by document number", the exact hazard a renumber-outside-this-screen orphans).
//!
//! **The one real fix, per the Build bullet**: `outstanding_amount = invoice_total - settled_
//! total`, computed and returned — the legacy "displays both figures and never compares them"
//! (§9.3's own closing note). No over-payment block: a negative `outstanding_amount` is a valid,
//! visible result, matching "port as-is, add clarity" rather than inventing a business rule.
//!
//! **Sorted by date** — a real `ORDER BY`, not the legacy's `UNION`-without-`ORDER-BY` accident
//! that always put every deposit slip before every cheque regardless of date (§9.2's note).
//!
//! **Judgment call, documented here**: no restriction to sales-only invoices (§9.5's "only sales
//! invoices can be settled" is listed as a *gap*, not a design to preserve, and this step's own
//! Build bullet/manual test never mentions it) — any inventory document type may be linked.

use crate::{auth::AuthUser, db, inventory_documents, AppState};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use chrono::NaiveDate;
use serde::Serialize;
use serde_json::{json, Value};

fn internal_error() -> (StatusCode, Json<Value>) {
    (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": "internal_error" })))
}
fn not_found(error: &str) -> (StatusCode, Json<Value>) {
    (StatusCode::NOT_FOUND, Json(json!({ "error": error })))
}

#[derive(sqlx::FromRow, Serialize)]
#[serde(rename_all = "camelCase")]
struct SettledInstrument {
    kind: String, // "deposit_slip" | "received_cheque"
    id: i64,
    date: NaiveDate,
    amount: i64,
    description: Option<String>,
    reference_number: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SettlementView {
    invoice_total: i64,
    settled_total: i64,
    /// `invoice_total - settled_total` — positive means still owed, negative means over-settled.
    /// No block, no warning colour here (that's a UI concern for 5.9) — just the real number the
    /// legacy screen never computed.
    outstanding_amount: i64,
    instruments: Vec<SettledInstrument>,
}

pub(crate) async fn get_settlement(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(document_id): Path<i64>,
) -> Result<Json<SettlementView>, (StatusCode, Json<Value>)> {
    let mut tx = db::begin(&state.pool, auth.tenant_id).await.map_err(|_| internal_error())?;

    let Some(document) = inventory_documents::fetch_document(&mut tx, auth.tenant_id, document_id)
        .await
        .map_err(|_| internal_error())?
    else {
        return Err(not_found("document_not_found"));
    };

    // source_module = 1 -> "inventory_invoice" (journal_sources, seeded by 2.3). UNION ALL is
    // correct here (unlike the legacy's plain UNION, §9.2's note) -- the two source tables can
    // never produce an identical row, so there is nothing to de-duplicate and no needless sort.
    let instruments: Vec<SettledInstrument> = sqlx::query_as(
        "SELECT 'deposit_slip' AS kind, id, slip_date AS date, amount, description, slip_number AS reference_number \
         FROM deposit_slips WHERE tenant_id = $1 AND source_module = 1 AND source_id = $2 \
         UNION ALL \
         SELECT 'received_cheque' AS kind, id, received_on AS date, amount, description, cheque_number AS reference_number \
         FROM received_cheques WHERE tenant_id = $1 AND source_module = 1 AND source_id = $2 \
         ORDER BY date",
    )
    .bind(auth.tenant_id)
    .bind(document_id)
    .fetch_all(&mut *tx)
    .await
    .map_err(|_| internal_error())?;

    tx.rollback().await.ok();

    let settled_total: i64 = instruments.iter().map(|i| i.amount).sum();
    let outstanding_amount = document.total_amount - settled_total;

    Ok(Json(SettlementView {
        invoice_total: document.total_amount,
        settled_total,
        outstanding_amount,
        instruments,
    }))
}
