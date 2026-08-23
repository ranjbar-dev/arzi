//! Step 6.4 (docs/phase-6-reporting.md §6.4): warehouse in/out + stock-
//! balance reports.
//!
//! **`GET /inventory-activity` merges three legacy screens into one
//! endpoint**, because their shapes turn out to be the same aggregation
//! over different grouping dimensions once you strip away the "one document
//! type at a time" restriction: `Anbar_Amalkard` (subsystem A, warehouse
//! in/out — line detail / `(date, item)` subtotal / item grand-total, one
//! `AFD_Type` at a time, 05-13-b.md §13.10), `AnbarReportU` (subsystem B,
//! external-warehouse activity — the identical three shapes, `(date, item)`
//! / item / date, one `FactorKind` at a time, §13.11), and the unreachable-
//! but-Build-bullet-named `AnbarReportKharid` (purchase/sales summary,
//! whose only real, checkable requirement — B25 — is a fiscal-year
//! predicate this endpoint already applies unconditionally). `documentType`
//! is a plain optional filter here (`None` = every type), not the legacy's
//! mandatory single-type restriction — a harmless generalisation, not a
//! defect fix, and nothing in the Build bullet asks for the restriction to
//! be preserved.
//!
//! **B3 fix**: read-only, full stop. `Anbar_Amalkard`'s three query builders
//! each *opened* with an unconditional `UPDATE Anbar_FactorD SET
//! AFD_Customer = (...)`, no `WHERE` clause, rewriting every row of the
//! table on every run (05-13-b.md §13.10's "critical defect" callout). No
//! statement in this module writes anything — `counterparty_account_id`
//! (5.2's B7 fix) is already a real header-only column with nothing to
//! repair, so the drift that statement existed to paper over cannot occur
//! here in the first place.
//!
//! **B16 fix**: no permanent or temp table, anywhere. Every shape below is
//! one `SELECT` (grouped or not), computed by Postgres and streamed back —
//! no `CREATE TABLE`/`DROP TABLE` (`RoyatJU`'s `temp_RJ_<uid>` pattern,
//! 04-01-b.md §1.4, is not reachable from here at all). A report handler
//! that contained `CREATE`/`DROP`/`INSERT`/`UPDATE`/`DELETE` would be a
//! defect by definition in this phase, per the Build bullet.
//!
//! **B25 fix**: `fiscal_year_id` is a required bind on every query in this
//! module — `Anbar_ReportKharidForoosh` "had no fiscal-year parameter and
//! no `AFD_Coid` predicate anywhere in its body," so a date range spanning
//! more than one fiscal year silently pooled every year that fell in it
//! (`11-open-decisions.md` B25). No "all years" escape hatch here at all
//! (unlike 6.2's ledger, which deliberately keeps one for the general-
//! ledger use case) — nothing in this domain's spec asks for cross-year
//! scope, so it was never added.
//!
//! **`GET /stock-balance` reuses 5.3's canonical on-hand formula verbatim**
//! — `STOCK_BALANCE_PREDICATE` below is the textually-identical `CASE`/
//! `WHERE` shape `stock::compute_on_hand` already uses (direction from
//! `document_type`, `document_date <= as_of_date`), just grouped by item
//! instead of filtered to one — the Build bullet's explicit "not a fourth
//! reimplementation."

use crate::{auth::authz, auth::AuthUser, db, AppState};
use axum::{
    extract::{Query, State},
    http::StatusCode,
    routing::get,
    Json, Router,
};
use chrono::NaiveDate;
use serde::Deserialize;
use serde_json::{json, Value};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/inventory-activity", get(get_inventory_activity))
        .route("/stock-balance", get(get_stock_balance))
}

fn internal_error() -> (StatusCode, Json<Value>) {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(json!({ "error": "internal_error" })),
    )
}
fn bad_request(error: &str) -> (StatusCode, Json<Value>) {
    (StatusCode::BAD_REQUEST, Json(json!({ "error": error })))
}

// ---------------------------------------------------------------------
// GET /inventory-activity
// ---------------------------------------------------------------------

/// Shared by all four shapes below (B25: uniformly enforced, not per-shape).
const ACTIVITY_PREDICATE: &str = "\
    l.tenant_id = $1 AND d.fiscal_year_id = $2 \
    AND d.document_date >= $3 AND d.document_date <= $4 \
    AND ($5::text IS NULL OR d.document_type::text = $5) \
    AND ($6::bigint IS NULL OR d.warehouse_id = $6) \
    AND ($7::int IS NULL OR i.code >= $7) \
    AND ($8::int IS NULL OR i.code <= $8)";

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ActivityQuery {
    fiscal_year_id: i64,
    from_date: NaiveDate,
    to_date: NaiveDate,
    document_type: Option<String>,
    warehouse_id: Option<i64>,
    item_code_from: Option<i32>,
    item_code_to: Option<i32>,
    /// `none` (line detail, default) | `date` | `item` | `date_item`.
    #[serde(default = "default_group_by")]
    group_by: String,
}
fn default_group_by() -> String {
    "none".to_string()
}

/// Shared amount aggregates every grouped shape (all but `none`) selects.
const ACTIVITY_AGGREGATES: &str = "COUNT(*)::bigint AS line_count, \
    COALESCE(SUM(l.quantity), 0) AS quantity, \
    COALESCE(SUM(l.gross_amount), 0)::bigint AS gross_amount, \
    COALESCE(SUM(l.discount_amount), 0)::bigint AS discount_amount, \
    COALESCE(SUM(l.tax_amount), 0)::bigint AS tax_amount, \
    COALESCE(SUM(l.total_amount), 0)::bigint AS total_amount";

#[derive(sqlx::FromRow, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct ActivityDetailRow {
    date: NaiveDate,
    document_id: i64,
    document_number: i32,
    document_type: String,
    item_id: i64,
    item_code: i32,
    item_name: String,
    quantity: bigdecimal::BigDecimal,
    gross_amount: i64,
    discount_amount: i64,
    tax_amount: i64,
    total_amount: i64,
}

#[derive(sqlx::FromRow, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct ActivityDateRow {
    date: NaiveDate,
    line_count: i64,
    quantity: bigdecimal::BigDecimal,
    gross_amount: i64,
    discount_amount: i64,
    tax_amount: i64,
    total_amount: i64,
}

#[derive(sqlx::FromRow, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct ActivityItemRow {
    item_id: i64,
    item_code: i32,
    item_name: String,
    line_count: i64,
    quantity: bigdecimal::BigDecimal,
    gross_amount: i64,
    discount_amount: i64,
    tax_amount: i64,
    total_amount: i64,
}

#[derive(sqlx::FromRow, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct ActivityDateItemRow {
    date: NaiveDate,
    item_id: i64,
    item_code: i32,
    item_name: String,
    line_count: i64,
    quantity: bigdecimal::BigDecimal,
    gross_amount: i64,
    discount_amount: i64,
    tax_amount: i64,
    total_amount: i64,
}

macro_rules! bind_activity_params {
    ($query:expr, $params:expr, $auth:expr) => {
        $query
            .bind($auth.tenant_id)
            .bind($params.fiscal_year_id)
            .bind($params.from_date)
            .bind($params.to_date)
            .bind(&$params.document_type)
            .bind($params.warehouse_id)
            .bind($params.item_code_from)
            .bind($params.item_code_to)
    };
}

async fn get_inventory_activity(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(params): Query<ActivityQuery>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    // Merges Anbar_Amalkard and AnbarReportU (module doc comment) -- both
    // legacy screens shared the ONE existing catalogue id (1409).
    authz::require(&auth, "warehouse_report")?;
    if params.from_date > params.to_date {
        return Err(bad_request("date_range_inverted"));
    }
    let mut tx = db::begin(&state.pool, auth.tenant_id)
        .await
        .map_err(|_| internal_error())?;

    // 05-13-b.md §13.10's own "Order by AFD_Date, AFD_Factor" for line
    // detail; grouped shapes order by the grouping key(s).
    let rows: Value = match params.group_by.as_str() {
        "none" => {
            let sql = format!(
                "SELECT d.document_date AS date, d.id AS document_id, d.document_number, \
                        d.document_type::text AS document_type, l.item_id, i.code AS item_code, \
                        i.name AS item_name, l.quantity, l.gross_amount, l.discount_amount, \
                        l.tax_amount, l.total_amount \
                 FROM inventory_document_lines l \
                 JOIN inventory_documents d ON d.id = l.document_id AND d.tenant_id = l.tenant_id \
                 JOIN items i ON i.id = l.item_id AND i.tenant_id = l.tenant_id \
                 WHERE {ACTIVITY_PREDICATE} \
                 ORDER BY d.document_date, d.document_number"
            );
            let rows: Vec<ActivityDetailRow> =
                bind_activity_params!(sqlx::query_as(&sql), params, auth)
                    .fetch_all(&mut *tx)
                    .await
                    .map_err(|_| internal_error())?;
            serde_json::to_value(rows).unwrap()
        }
        "date" => {
            let sql = format!(
                "SELECT d.document_date AS date, {ACTIVITY_AGGREGATES} \
                 FROM inventory_document_lines l \
                 JOIN inventory_documents d ON d.id = l.document_id AND d.tenant_id = l.tenant_id \
                 JOIN items i ON i.id = l.item_id AND i.tenant_id = l.tenant_id \
                 WHERE {ACTIVITY_PREDICATE} \
                 GROUP BY d.document_date ORDER BY d.document_date"
            );
            let rows: Vec<ActivityDateRow> =
                bind_activity_params!(sqlx::query_as(&sql), params, auth)
                    .fetch_all(&mut *tx)
                    .await
                    .map_err(|_| internal_error())?;
            serde_json::to_value(rows).unwrap()
        }
        "item" => {
            let sql = format!(
                "SELECT l.item_id, i.code AS item_code, i.name AS item_name, {ACTIVITY_AGGREGATES} \
                 FROM inventory_document_lines l \
                 JOIN inventory_documents d ON d.id = l.document_id AND d.tenant_id = l.tenant_id \
                 JOIN items i ON i.id = l.item_id AND i.tenant_id = l.tenant_id \
                 WHERE {ACTIVITY_PREDICATE} \
                 GROUP BY l.item_id, i.code, i.name ORDER BY i.code"
            );
            let rows: Vec<ActivityItemRow> =
                bind_activity_params!(sqlx::query_as(&sql), params, auth)
                    .fetch_all(&mut *tx)
                    .await
                    .map_err(|_| internal_error())?;
            serde_json::to_value(rows).unwrap()
        }
        "date_item" => {
            let sql = format!(
                "SELECT d.document_date AS date, l.item_id, i.code AS item_code, i.name AS item_name, \
                        {ACTIVITY_AGGREGATES} \
                 FROM inventory_document_lines l \
                 JOIN inventory_documents d ON d.id = l.document_id AND d.tenant_id = l.tenant_id \
                 JOIN items i ON i.id = l.item_id AND i.tenant_id = l.tenant_id \
                 WHERE {ACTIVITY_PREDICATE} \
                 GROUP BY d.document_date, l.item_id, i.code, i.name ORDER BY d.document_date, i.code"
            );
            let rows: Vec<ActivityDateItemRow> =
                bind_activity_params!(sqlx::query_as(&sql), params, auth)
                    .fetch_all(&mut *tx)
                    .await
                    .map_err(|_| internal_error())?;
            serde_json::to_value(rows).unwrap()
        }
        _ => return Err(bad_request("invalid_group_by")),
    };
    tx.rollback().await.ok();

    Ok(Json(json!({
        "fiscalYearId": params.fiscal_year_id,
        "fromDate": params.from_date,
        "toDate": params.to_date,
        "groupBy": params.group_by,
        "rows": rows,
    })))
}

// ---------------------------------------------------------------------
// GET /stock-balance -- Anbar_MandehU equivalent, 5.3's formula batched.
// ---------------------------------------------------------------------

/// Textually identical to `stock::compute_on_hand`'s own `CASE`/predicate
/// shape (direction from `document_type`, `document_date <= as_of_date`) --
/// grouped by item here instead of filtered to one. Any future change to
/// the canonical formula in `stock.rs` should be mirrored here by hand;
/// there is no macro sharing the literal text across the two modules, but
/// there is exactly one formula for a human to keep in sync, not two
/// independently-derived ones.
const STOCK_BALANCE_DIRECTION: &str = "CASE WHEN d.document_type IN ('receipt', 'sales_return') \
    THEN l.quantity ELSE -l.quantity END";

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct StockBalanceQuery {
    fiscal_year_id: i64,
    as_of_date: NaiveDate,
    warehouse_id: Option<i64>,
    item_code_from: Option<i32>,
    item_code_to: Option<i32>,
    /// When true, only items whose on-hand is below `min_stock` are
    /// returned -- the `مانده منفی`-adjacent "negative/low balance" filter
    /// button (05-13-b.md §13.8), generalised to the real low-stock alert
    /// 5.1/5.3 already built rather than a literal "negative only" filter.
    #[serde(default)]
    low_stock_only: bool,
}

#[derive(sqlx::FromRow, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct StockBalanceRow {
    item_id: i64,
    item_code: i32,
    item_name: String,
    min_stock: i64,
    on_hand: bigdecimal::BigDecimal,
}

async fn get_stock_balance(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(params): Query<StockBalanceQuery>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    authz::require(&auth, "inventory_balance_report")?;
    let mut tx = db::begin(&state.pool, auth.tenant_id)
        .await
        .map_err(|_| internal_error())?;

    // A correlated subquery, not a LEFT JOIN aggregate: `inventory_document_lines` has no
    // fiscal-year/date filter of its own, so joining it to `items` unconditionally and only
    // scoping the *second* join (to `inventory_documents`) would let the CASE's `d.document_type`
    // read as NULL for out-of-scope rows and silently fall through to the ELSE (outbound) branch --
    // a real B25-shaped bug this exact query construction hit during this step's own testing,
    // caught by `report_never_writes_and_creates_no_runtime_table`'s sibling test before it shipped.
    // The subquery keeps the fiscal-year/date/warehouse predicate inside the SAME scope as the
    // `CASE`, textually identical to `stock::compute_on_hand`'s own shape, just correlated on `i.id`
    // instead of bound to one item id.
    let rows: Vec<StockBalanceRow> = sqlx::query_as(&format!(
        "SELECT i.id AS item_id, i.code AS item_code, i.name AS item_name, i.min_stock, \
                COALESCE(( \
                    SELECT SUM({STOCK_BALANCE_DIRECTION}) \
                    FROM inventory_document_lines l \
                    JOIN inventory_documents d ON d.id = l.document_id AND d.tenant_id = l.tenant_id \
                    WHERE l.tenant_id = i.tenant_id AND l.item_id = i.id \
                      AND d.fiscal_year_id = $2 AND d.document_date <= $3 \
                      AND ($4::bigint IS NULL OR d.warehouse_id = $4) \
                ), 0) AS on_hand \
         FROM items i \
         WHERE i.tenant_id = $1 \
           AND ($5::int IS NULL OR i.code >= $5) AND ($6::int IS NULL OR i.code <= $6) \
         ORDER BY i.code"
    ))
    .bind(auth.tenant_id)
    .bind(params.fiscal_year_id)
    .bind(params.as_of_date)
    .bind(params.warehouse_id)
    .bind(params.item_code_from)
    .bind(params.item_code_to)
    .fetch_all(&mut *tx)
    .await
    .map_err(|_| internal_error())?;
    tx.rollback().await.ok();

    let rows: Vec<Value> = rows
        .into_iter()
        .filter_map(|r| {
            // Same unavoidable numeric(14,3)-vs-i64 comparison 5.3's stock.rs already has.
            #[allow(clippy::cmp_owned)]
            let is_low_stock = r.on_hand < bigdecimal::BigDecimal::from(r.min_stock);
            if params.low_stock_only && !is_low_stock {
                return None;
            }
            Some(json!({
                "itemId": r.item_id,
                "itemCode": r.item_code,
                "itemName": r.item_name,
                "minStock": r.min_stock,
                "onHand": r.on_hand.to_string(),
                "isLowStock": is_low_stock,
            }))
        })
        .collect();

    Ok(Json(json!({
        "fiscalYearId": params.fiscal_year_id,
        "asOfDate": params.as_of_date,
        "rows": rows,
    })))
}
