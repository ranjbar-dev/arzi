//! Step 5.3 (docs/phase-5-inventory.md §5.3): one canonical on-hand formula — the legacy has
//! three that disagree (specs/05-inventory/05-05-a.md §5.1). This module is the single place that
//! computes it; both the on-hand endpoint and the stock card in `items.rs` call into it, per
//! §11.4 requirement #1 ("one balance function ... everything else is a call to it").
//!
//! **Canonical formula, per the Build bullet**: the `Anbar_Mandeh` report's shape (§5.1.2) —
//! direction from document type (`receipt`/`sales_return` inbound, `issue`/`purchase_return`
//! outbound, quantities always stored positive), a real date-windowed cumulative sum, not the
//! inferior line-lookup implementation's whole-year-no-window version (§5.1.3). Real `date`
//! comparisons throughout (5.2 already stores `document_date` as a Postgres `date`, not the
//! legacy's zero-padded Jalali string relying on lexicographic sort, §5.1.2's own note).
//!
//! **§11.4 requirement #2 ("a stable, immutable movement sequence") is already satisfied by 5.2's
//! own architecture, not something this step has to add**: `inventory_document_lines.id` never
//! changes across an edit, because 5.2 chose incremental per-line CRUD over the legacy's
//! delete-and-reinsert-every-line-on-save model. So `id` is exactly the tie-breaker §11.1.3's
//! "the ordering question" says the legacy schema cannot provide — used below as the same-date
//! sort key for the stock card's running balance.
//!
//! **Judgment call, documented here**: on-hand counts every `inventory_document_lines` row
//! regardless of its document's `status` (draft/posted/frozen) — matching the legacy's own
//! "existence = counted" behaviour exactly (subsystem A had no status column at all, so a saved
//! row always counted). This is deliberate, not an oversight: at this point in the roadmap
//! `status` can only ever be `'draft'` (5.8 is what ever produces `'posted'`), so filtering to
//! non-draft would make on-hand permanently zero until 5.8 lands, which would make this step's own
//! manual test unrunnable. Revisit only if a future step needs draft documents excluded.
//!
//! ---
//!
//! Step 5.4 (docs/phase-5-inventory.md §5.4) added `compute_average_cost` below — the *one*
//! weighted-average-of-purchases figure, replacing the legacy's two disagreeing implementations
//! (specs/05-inventory/05-06-a/b.md §6.2/§6.3). Deliberately the `Anbar_Mandeh`-style formula
//! (numerator = the stored `gross_amount`, a date-windowed cumulative sum via the *same*
//! `document_date <= as_of_date` shape `compute_on_hand` already uses) rather than
//! `Anbar_Jens_Phi1`'s whole-fiscal-year live-recompute — "matching 5.3's canonical stock formula,
//! for consistency" is the Build bullet's own words. **There is no automatic costing engine and
//! this step doesn't invent one** (§6.0's own headline: unit price stays manual entry everywhere;
//! this is an advisory suggestion only, never written back automatically). Ported exactly per
//! §6.8's summary: purchases only (`document_type = 'receipt'`), discounts and tax excluded from
//! the numerator by construction (gross, not net), zero when there are no purchases in the window.
//! Rounded correctly (`money::round_to_rial`, this project's one round-half-away-from-zero
//! decision — see that module's doc comment) at the one point the figure is offered to the
//! operator — not the legacy's silent per-line truncation (§6.1's "systematic downward bias"
//! finding) — though the worked example in the manual test happens to divide evenly (11,800,000 ÷
//! 200 = 59,000.0 exactly), so rounding vs. truncation don't actually diverge there.
//! **Judgment call**: `warehouse_id` is accepted as an optional scope, reusing `compute_on_hand`'s
//! own parameter shape — the legacy has no warehouse dimension for costing at all ("one global
//! average per item code across all warehouses"), but since 5.1 already gave the schema a real
//! warehouse relationship and `compute_on_hand` already exposes the same optional filter, adding
//! it here too costs nothing and stays consistent; omitting it (`None`) reproduces the legacy's
//! unscoped behaviour exactly.

use bigdecimal::BigDecimal;
use chrono::NaiveDate;
use serde::Serialize;
use sqlx::{Postgres, Transaction};

/// The one balance function (§11.4 #1). `warehouse_id: None` sums across every warehouse the item
/// is assigned to (5.1's real `item_warehouses` relationship makes per-warehouse scoping possible
/// at all — subsystem A had "one global stock pool per item code" because it had no such
/// relationship to scope by). `exclude_document_id` is 5.3's manual test #3 — excluding a
/// document's own lines while it's being edited, done by real id always, never the legacy's
/// "empty string matches nothing on a new invoice" accident (§5.1.3's own finding).
pub async fn compute_on_hand(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: i64,
    item_id: i64,
    fiscal_year_id: i64,
    warehouse_id: Option<i64>,
    as_of_date: NaiveDate,
    exclude_document_id: Option<i64>,
) -> Result<BigDecimal, sqlx::Error> {
    sqlx::query_scalar(
        "SELECT COALESCE(SUM( \
             CASE WHEN d.document_type IN ('receipt', 'sales_return') THEN l.quantity ELSE -l.quantity END \
         ), 0) \
         FROM inventory_document_lines l \
         JOIN inventory_documents d ON d.id = l.document_id AND d.tenant_id = l.tenant_id \
         WHERE l.tenant_id = $1 AND l.item_id = $2 AND d.fiscal_year_id = $3 AND d.document_date <= $4 \
           AND ($5::bigint IS NULL OR d.warehouse_id = $5) \
           AND ($6::bigint IS NULL OR d.id <> $6)",
    )
    .bind(tenant_id)
    .bind(item_id)
    .bind(fiscal_year_id)
    .bind(as_of_date)
    .bind(warehouse_id)
    .bind(exclude_document_id)
    .fetch_one(&mut **tx)
    .await
}

#[derive(sqlx::FromRow)]
struct MovementRow {
    line_id: i64,
    document_id: i64,
    document_type: String,
    document_number: i32,
    document_date: NaiveDate,
    quantity: BigDecimal,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StockCardMovement {
    pub line_id: i64,
    pub document_id: i64,
    pub document_type: String,
    pub document_number: i32,
    pub document_date: NaiveDate,
    pub quantity_signed: BigDecimal,
    pub running_balance: BigDecimal,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StockCard {
    pub opening_balance: BigDecimal,
    pub movements: Vec<StockCardMovement>,
}

/// §11.4 requirement #5: the opening balance is explicit, not zero — `compute_on_hand` as of the
/// day before `from_date`, the same function every other on-hand figure in the system uses.
/// Requirement #3 ("the running balance belongs in the query, not the renderer" — §11.1.3's five
/// consequences of the legacy computing it in a FastReport `PascalScript` at render time):
/// accumulated here in Rust from a single date/id-ordered query, still one deterministic pass over
/// real data, not per-print-traversal arithmetic that could double-count on a second render pass.
pub async fn compute_stock_card(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: i64,
    item_id: i64,
    fiscal_year_id: i64,
    warehouse_id: Option<i64>,
    from_date: NaiveDate,
    to_date: NaiveDate,
) -> Result<StockCard, sqlx::Error> {
    let opening_balance = match from_date.pred_opt() {
        Some(day_before) => {
            compute_on_hand(tx, tenant_id, item_id, fiscal_year_id, warehouse_id, day_before, None).await?
        }
        None => BigDecimal::from(0), // from_date is the earliest representable date -- nothing precedes it
    };

    let rows: Vec<MovementRow> = sqlx::query_as(
        "SELECT l.id AS line_id, d.id AS document_id, d.document_type::text AS document_type, \
                d.document_number, d.document_date, l.quantity \
         FROM inventory_document_lines l \
         JOIN inventory_documents d ON d.id = l.document_id AND d.tenant_id = l.tenant_id \
         WHERE l.tenant_id = $1 AND l.item_id = $2 AND d.fiscal_year_id = $3 \
           AND d.document_date BETWEEN $4 AND $5 \
           AND ($6::bigint IS NULL OR d.warehouse_id = $6) \
         ORDER BY d.document_date, l.id", // l.id is the stable sequence (see module doc comment)
    )
    .bind(tenant_id)
    .bind(item_id)
    .bind(fiscal_year_id)
    .bind(from_date)
    .bind(to_date)
    .bind(warehouse_id)
    .fetch_all(&mut **tx)
    .await?;

    let mut running = opening_balance.clone();
    let mut movements = Vec::with_capacity(rows.len());
    for row in rows {
        let signed = if matches!(row.document_type.as_str(), "receipt" | "sales_return") {
            row.quantity.clone()
        } else {
            -row.quantity.clone()
        };
        running += &signed;
        movements.push(StockCardMovement {
            line_id: row.line_id,
            document_id: row.document_id,
            document_type: row.document_type,
            document_number: row.document_number,
            document_date: row.document_date,
            quantity_signed: signed,
            running_balance: running.clone(),
        });
    }

    Ok(StockCard { opening_balance, movements })
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AverageCost {
    /// Zero when there are no purchases in the window (§6.8: "0 when there are no purchases") —
    /// never `sale_price`, and never any other fallback.
    pub average_cost: i64,
    pub purchase_quantity: BigDecimal,
}

/// §6.8's exact formula: `trunc(Σ(quantity × unit_price) over purchases ÷ Σ quantity)`, ported as
/// `Σ gross_amount ÷ Σ quantity` (the `Anbar_Mandeh`-style stored-column numerator, per this
/// module's own doc comment) — correctly rounded here, not truncated, since nothing in this step's
/// spec asks for the legacy's truncation bug to be reproduced at the point of display.
pub async fn compute_average_cost(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: i64,
    item_id: i64,
    fiscal_year_id: i64,
    warehouse_id: Option<i64>,
    as_of_date: NaiveDate,
    exclude_document_id: Option<i64>,
) -> Result<AverageCost, sqlx::Error> {
    let (gross_sum, quantity_sum): (i64, BigDecimal) = sqlx::query_as(
        "SELECT COALESCE(SUM(l.gross_amount), 0)::bigint, COALESCE(SUM(l.quantity), 0) \
         FROM inventory_document_lines l \
         JOIN inventory_documents d ON d.id = l.document_id AND d.tenant_id = l.tenant_id \
         WHERE l.tenant_id = $1 AND l.item_id = $2 AND d.fiscal_year_id = $3 \
           AND d.document_type = 'receipt' AND d.document_date <= $4 \
           AND ($5::bigint IS NULL OR d.warehouse_id = $5) \
           AND ($6::bigint IS NULL OR d.id <> $6)",
    )
    .bind(tenant_id)
    .bind(item_id)
    .bind(fiscal_year_id)
    .bind(as_of_date)
    .bind(warehouse_id)
    .bind(exclude_document_id)
    .fetch_one(&mut **tx)
    .await?;

    let average_cost = if quantity_sum.sign() == bigdecimal::num_bigint::Sign::Plus {
        crate::money::round_to_rial(&(BigDecimal::from(gross_sum) / &quantity_sum))
    } else {
        0
    };

    Ok(AverageCost { average_cost, purchase_quantity: quantity_sum })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn average_cost_rounding_matches_manual_test_worked_example() {
        // 100@50,000 + 60@65,000 + 40@72,500 -> gross 11,800,000 over 200 units -> 59,000 exactly.
        let gross = 100 * 50_000 + 60 * 65_000 + 40 * 72_500i64;
        let quantity = BigDecimal::from(200);
        let avg = crate::money::round_to_rial(&(BigDecimal::from(gross) / &quantity));
        assert_eq!(avg, 59_000);
    }
}
