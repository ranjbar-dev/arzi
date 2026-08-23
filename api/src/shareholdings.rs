//! Step 3.3 (docs/phase-3-parties.md §3.3): shareholder equity. The legacy
//! has none of this (specs/07-parties-and-shareholders/07-05.md's exhaustive
//! "derivation of absence") — A4 (11-open-decisions.md) already decided this
//! is in scope and genuinely new, not a port; this module designs it fresh,
//! with its own worked example (below, mirrored in the unit tests) rather
//! than reverse-engineering the external Saham.Dbo product.
//!
//! **Judgment call, documented here since the manual test explicitly asks
//! for one:** profit distribution *reproportions* among the shareholders
//! active during the requested fiscal year — it does not hold the excluded
//! holder's share unallocated. "Distribute proportionally to each active
//! shareholder's ownership_percentage **as of that year**" (the Build
//! bullet's own wording) only makes sense if percentages are recomputed
//! over the active-that-year subset; leaving a gap unallocated would mean
//! the year's profit doesn't actually get fully distributed, which is a
//! worse outcome nothing in the spec asks for.
//!
//! **Active during a fiscal year**: `join_date <= year.end_date AND
//! (exit_date IS NULL OR exit_date >= year.start_date)` — an overlap test
//! between the holding's active span and the fiscal year, not a snapshot at
//! a single instant. No day-count proration of the allocation itself for a
//! partial-year holding — nothing in the Build bullet or manual test asks
//! for that, and it would be new arithmetic invented, not derived.
//!
//! **Rounding**: allocations are computed with the largest-remainder method
//! (integer floor-division per holding, then the few leftover rials handed
//! one each to the holdings with the largest truncated fraction) so the
//! allocations always sum to *exactly* `profit_amount` — money is `i64`
//! rials end-to-end, never floating point (docs/00-overview.md).
//! `ownership_percentage` itself is a display-only ratio, not money, and IS
//! an `f64` — nothing downstream sums or compares it as currency.

use crate::{audit, auth::AuthUser, db, AppState};
use axum::{
    extract::{Path, State},
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
        .route("/", get(list_shareholdings).post(create_shareholding))
        .route("/{id}", axum::routing::put(update_shareholding))
        .route("/profit-distribution", post(profit_distribution))
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

#[derive(sqlx::FromRow, Clone)]
struct HoldingRow {
    id: i64,
    party_id: i64,
    share_count: i64,
    nominal_value: i64,
    join_date: NaiveDate,
    exit_date: Option<NaiveDate>,
}

const HOLDING_COLUMNS: &str = "id, party_id, share_count, nominal_value, join_date, exit_date";

async fn fetch_all(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: i64,
) -> Result<Vec<HoldingRow>, sqlx::Error> {
    sqlx::query_as(&format!(
        "SELECT {HOLDING_COLUMNS} FROM shareholdings WHERE tenant_id = $1 ORDER BY id"
    ))
    .bind(tenant_id)
    .fetch_all(&mut **tx)
    .await
}

// ---- list (with the display-only ownership_percentage) --------------------

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct HoldingView {
    id: i64,
    party_id: i64,
    share_count: i64,
    nominal_value: i64,
    join_date: NaiveDate,
    exit_date: Option<NaiveDate>,
    ownership_percentage: f64,
}

async fn list_shareholdings(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Vec<HoldingView>>, (StatusCode, Json<Value>)> {
    let mut tx = db::begin(&state.pool, auth.tenant_id)
        .await
        .map_err(|_| internal_error())?;
    let rows = fetch_all(&mut tx, auth.tenant_id)
        .await
        .map_err(|_| internal_error())?;
    tx.rollback().await.ok();

    let total: i64 = rows.iter().map(|r| r.share_count).sum();
    let views = rows
        .into_iter()
        .map(|r| {
            let ownership_percentage = if total > 0 {
                (r.share_count as f64) / (total as f64) * 100.0
            } else {
                0.0
            };
            HoldingView {
                id: r.id,
                party_id: r.party_id,
                share_count: r.share_count,
                nominal_value: r.nominal_value,
                join_date: r.join_date,
                exit_date: r.exit_date,
                ownership_percentage,
            }
        })
        .collect();
    Ok(Json(views))
}

// ---- create / update -------------------------------------------------------

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct HoldingFields {
    party_id: i64,
    share_count: i64,
    nominal_value: Option<i64>,
    join_date: NaiveDate,
    exit_date: Option<NaiveDate>,
}

fn validate_fields(f: &HoldingFields) -> Result<(), &'static str> {
    if f.share_count <= 0 {
        return Err("invalid_share_count");
    }
    if let Some(exit) = f.exit_date {
        if exit < f.join_date {
            return Err("exit_before_join");
        }
    }
    Ok(())
}

async fn create_shareholding(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<HoldingFields>,
) -> Result<(StatusCode, Json<Value>), (StatusCode, Json<Value>)> {
    validate_fields(&req).map_err(bad_request)?;
    let mut tx = db::begin(&state.pool, auth.tenant_id)
        .await
        .map_err(|_| internal_error())?;

    let party_exists: bool =
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM parties WHERE id = $1)")
            .bind(req.party_id)
            .fetch_one(&mut *tx)
            .await
            .map_err(|_| internal_error())?;
    if !party_exists {
        return Err(not_found("party"));
    }

    let id: i64 = sqlx::query_scalar(
        "INSERT INTO shareholdings (tenant_id, party_id, share_count, nominal_value, join_date, \
         exit_date, created_by) VALUES ($1, $2, $3, $4, $5, $6, $7) RETURNING id",
    )
    .bind(auth.tenant_id)
    .bind(req.party_id)
    .bind(req.share_count)
    .bind(req.nominal_value.unwrap_or(0))
    .bind(req.join_date)
    .bind(req.exit_date)
    .bind(auth.user_id)
    .fetch_one(&mut *tx)
    .await
    .map_err(|_| internal_error())?;

    audit::record_mutation(
        &mut tx,
        auth.tenant_id,
        "shareholdings",
        id,
        "insert",
        Some(auth.user_id),
        None,
        Some(json!({ "partyId": req.party_id, "shareCount": req.share_count })),
    )
    .await
    .map_err(|_| internal_error())?;

    tx.commit().await.map_err(|_| internal_error())?;
    Ok((StatusCode::CREATED, Json(json!({ "id": id }))))
}

async fn update_shareholding(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
    Json(req): Json<HoldingFields>,
) -> Result<StatusCode, (StatusCode, Json<Value>)> {
    validate_fields(&req).map_err(bad_request)?;
    let mut tx = db::begin(&state.pool, auth.tenant_id)
        .await
        .map_err(|_| internal_error())?;

    let existing: Option<(i64, Option<NaiveDate>)> =
        sqlx::query_as("SELECT share_count, exit_date FROM shareholdings WHERE id = $1")
            .bind(id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(|_| internal_error())?;
    let Some((old_share_count, old_exit_date)) = existing else {
        return Err(not_found("shareholding"));
    };

    sqlx::query(
        "UPDATE shareholdings SET share_count = $1, nominal_value = $2, join_date = $3, \
         exit_date = $4, updated_at = now(), updated_by = $5 WHERE id = $6",
    )
    .bind(req.share_count)
    .bind(req.nominal_value.unwrap_or(0))
    .bind(req.join_date)
    .bind(req.exit_date)
    .bind(auth.user_id)
    .bind(id)
    .execute(&mut *tx)
    .await
    .map_err(|_| internal_error())?;

    audit::record_mutation(
        &mut tx,
        auth.tenant_id,
        "shareholdings",
        id,
        "update",
        Some(auth.user_id),
        Some(json!({ "shareCount": old_share_count, "exitDate": old_exit_date })),
        Some(json!({ "shareCount": req.share_count, "exitDate": req.exit_date })),
    )
    .await
    .map_err(|_| internal_error())?;

    tx.commit().await.map_err(|_| internal_error())?;
    Ok(StatusCode::NO_CONTENT)
}

// ---- profit distribution ---------------------------------------------------

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ProfitDistributionRequest {
    fiscal_year_id: i64,
    profit_amount: i64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Allocation {
    shareholding_id: i64,
    party_id: i64,
    share_count: i64,
    percentage: f64,
    allocation: i64,
}

/// A shareholding is active during `[year_start, year_end]` when its own
/// active span overlaps that range at all.
fn active_during(holding: &HoldingRow, year_start: NaiveDate, year_end: NaiveDate) -> bool {
    holding.join_date <= year_end && holding.exit_date.is_none_or(|exit| exit >= year_start)
}

/// Largest-remainder integer distribution — see the module doc comment.
fn distribute(profit_amount: i64, holdings: &[HoldingRow]) -> Vec<i64> {
    let total_shares: i128 = holdings.iter().map(|h| h.share_count as i128).sum();
    let profit = profit_amount as i128;

    let mut allocations: Vec<i64> = holdings
        .iter()
        .map(|h| (profit * h.share_count as i128 / total_shares) as i64)
        .collect();

    let distributed: i64 = allocations.iter().sum();
    let mut remaining = profit_amount - distributed;

    let mut by_fraction: Vec<usize> = (0..holdings.len()).collect();
    by_fraction.sort_by(|&a, &b| {
        let frac = |i: usize| (profit * holdings[i].share_count as i128) % total_shares;
        frac(b).cmp(&frac(a))
    });
    for &i in &by_fraction {
        if remaining <= 0 {
            break;
        }
        allocations[i] += 1;
        remaining -= 1;
    }

    allocations
}

async fn profit_distribution(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<ProfitDistributionRequest>,
) -> Result<Json<Vec<Allocation>>, (StatusCode, Json<Value>)> {
    if req.profit_amount < 0 {
        return Err(bad_request("invalid_profit_amount"));
    }
    let mut tx = db::begin(&state.pool, auth.tenant_id)
        .await
        .map_err(|_| internal_error())?;

    let year: Option<(NaiveDate, NaiveDate)> =
        sqlx::query_as("SELECT start_date, end_date FROM fiscal_years WHERE id = $1")
            .bind(req.fiscal_year_id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(|_| internal_error())?;
    let Some((year_start, year_end)) = year else {
        return Err(not_found("fiscal_year"));
    };

    let all_holdings = fetch_all(&mut tx, auth.tenant_id)
        .await
        .map_err(|_| internal_error())?;
    tx.rollback().await.ok();

    let active: Vec<HoldingRow> = all_holdings
        .into_iter()
        .filter(|h| active_during(h, year_start, year_end))
        .collect();
    if active.is_empty() {
        return Err(bad_request("no_active_shareholders"));
    }
    let total_active_shares: i64 = active.iter().map(|h| h.share_count).sum();

    let allocations = distribute(req.profit_amount, &active);
    let result = active
        .iter()
        .zip(allocations)
        .map(|(h, allocation)| Allocation {
            shareholding_id: h.id,
            party_id: h.party_id,
            share_count: h.share_count,
            percentage: (h.share_count as f64) / (total_active_shares as f64) * 100.0,
            allocation,
        })
        .collect();

    Ok(Json(result))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn holding(
        id: i64,
        party_id: i64,
        share_count: i64,
        join: &str,
        exit: Option<&str>,
    ) -> HoldingRow {
        HoldingRow {
            id,
            party_id,
            share_count,
            nominal_value: 0,
            join_date: NaiveDate::parse_from_str(join, "%Y-%m-%d").unwrap(),
            exit_date: exit.map(|s| NaiveDate::parse_from_str(s, "%Y-%m-%d").unwrap()),
        }
    }

    /// The manual test's worked example: 500/300/200 of 1000 total,
    /// profit 100,000,000 -> 50M/30M/20M exactly.
    #[test]
    fn distributes_the_worked_example_exactly() {
        let holdings = vec![
            holding(1, 10, 500, "2020-01-01", None),
            holding(2, 11, 300, "2020-01-01", None),
            holding(3, 12, 200, "2020-01-01", None),
        ];
        let allocations = distribute(100_000_000, &holdings);
        assert_eq!(allocations, vec![50_000_000, 30_000_000, 20_000_000]);
        assert_eq!(allocations.iter().sum::<i64>(), 100_000_000);
    }

    /// A non-exact split still sums to exactly the profit figure (largest-
    /// remainder method) — 100 split 3 ways can't divide evenly.
    #[test]
    fn rounding_remainder_always_sums_exactly() {
        let holdings = vec![
            holding(1, 10, 1, "2020-01-01", None),
            holding(2, 11, 1, "2020-01-01", None),
            holding(3, 12, 1, "2020-01-01", None),
        ];
        let allocations = distribute(100, &holdings);
        assert_eq!(allocations.iter().sum::<i64>(), 100);
        // Each gets 33, one gets the leftover rial -> 34.
        assert_eq!(allocations.iter().filter(|&&a| a == 33).count(), 2);
        assert_eq!(allocations.iter().filter(|&&a| a == 34).count(), 1);
    }

    /// Manual test #3's judgment call: an exited shareholder is excluded and
    /// the remaining two are reproportioned (50/300+200*100=60%, 40%), not
    /// left with an unallocated gap.
    #[test]
    fn excluded_shareholder_is_reproportioned_not_left_unallocated() {
        let all = vec![
            holding(1, 10, 500, "2020-01-01", Some("2020-06-01")), // exits before FY 1400
            holding(2, 11, 300, "2020-01-01", None),
            holding(3, 12, 200, "2020-01-01", None),
        ];
        let year_start = NaiveDate::parse_from_str("2021-03-21", "%Y-%m-%d").unwrap();
        let year_end = NaiveDate::parse_from_str("2022-03-20", "%Y-%m-%d").unwrap();
        let active: Vec<_> = all
            .into_iter()
            .filter(|h| active_during(h, year_start, year_end))
            .collect();
        assert_eq!(active.len(), 2);

        let allocations = distribute(100_000_000, &active);
        // 300:200 of the remaining 500 -> 60,000,000 / 40,000,000.
        assert_eq!(allocations, vec![60_000_000, 40_000_000]);
        assert_eq!(allocations.iter().sum::<i64>(), 100_000_000);
    }
}
