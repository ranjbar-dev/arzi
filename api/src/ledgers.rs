//! Step 6.2 (docs/phase-6-reporting.md §6.2): the general ledger (Daftar
//! Kol) and subsidiary ledger (Daftar Moein), merged into **one** endpoint
//! per specs/04-reporting/04-03-b.md §3.3's own "merge verdict" — a
//! structured account filter (an exact 4-segment coordinate, a partial
//! coordinate/subtree, or an explicit account-id list) replaces `DKolU` +
//! `DMoein` + `TMoein`'s three separately-coded screens.
//!
//! **B6 fix — the core structural change.** The legacy general ledger
//! (`DKolU`) reads only `M_Kind = 2` rows: lines of a *manually generated*
//! journal-summary voucher, so "Daftar Kol shows nothing until someone
//! presses ساخت روزنامه" (04-03-a.md §3.0). This rebuild's ledger reads
//! posted (`status = 'posted'`) `voucher_lines` **directly** — no dependency
//! on journal generation (2.6) having ever run. `kind = 'ledger'` is filtered
//! too (undocumented in the Build bullet but structurally required): a
//! journal-generated summary line posts against the very same Kol-level
//! account its own source lines already hit, so including both `kind`s
//! would double-count every summarised Kol — same reasoning as 6.1's
//! `trial_balance.rs`.
//!
//! **Posted-only, uniformly** — a real behaviour *change* from the legacy
//! (04-03-a.md §3.1.d / §3.6: "all four ledgers include state-0 drafts"),
//! per the Build bullet's own "matching `M_kind=1`-equivalent (ordinary
//! **posted** lines)" wording for the subsidiary ledger, extended here to
//! the general ledger too for the same reason 6.1 applied A8 uniformly.
//!
//! **B4 fix, structural, not a checked invariant**: the opening leg and the
//! movement leg are built from the exact same Rust `&str` constant
//! (`ACCOUNT_AND_YEAR_PREDICATE`), spliced verbatim into both queries by
//! `fetch_opening`/`fetch_movement` below — the legacy's defect (`TMoein`'s
//! opening leg omits `M_kind = 1` while the movement leg includes it,
//! 04-03-b.md §3.3(b)) requires two independently-maintained SQL fragments
//! to drift apart in the first place; here there is only one fragment to
//! drift.
//!
//! **B5 fix, applied everywhere an opening boundary exists**: `line_date <
//! from_date`, strictly less-than, in `fetch_opening` — reused unmodified by
//! both the ledger endpoint and the party-balance-list endpoint below (the
//! `BedBes` equivalent, whose own `Rem1` used `<= D1`, one day out of step
//! with every ledger, 04-03-b.md §3.3 / `11-open-decisions.md` B5).
//!
//! **Opening balance netted, not two gross columns**: unlike the legacy's
//! `DKolU`/`DMoein` opening row (`Sum(M_Bed)`, `Sum(M_Bes)` side by side,
//! never netted — 04-03-a.md §3.1.a), `fetch_opening` returns one signed,
//! credit-positive figure, matching the running balance's own sign
//! convention (`{ amount, side }`, never a parallel grid/print duplication).
//!
//! **Ordering tie-break fixed** (04-03-a.md §3.1.c): `ORDER BY line_date,
//! voucher_number, voucher_lines.id` — the legacy has no tie-break below the
//! voucher number at all.
//!
//! **Permission gate**: `Is_Admin_Or_Valid_Daftar`'s real rule is "no
//! segment of the account is locked", not "admin only" (04-03-a.md §3.1's
//! own note that the legacy's refusal message is wrong about its own rule)
//! — `any_segment_locked` below implements the real rule; a *missing*
//! `accounts` row is never locked (matches the legacy exactly).
//!
//! **Cross-fiscal-year view**: `fiscal_year_id` is `Option<i64>` — `None`
//! reproduces the legacy's synthetic `CO_ID = 0` ("all fiscal periods")
//! row, an explicit typed parameter rather than a magic sentinel threaded
//! through string-built SQL.
//!
//! **B5's other half — the party-balance list (`BedBes`/R09 equivalent)**:
//! 6.2's own Build bullet names this report by name as somewhere B5 must be
//! fixed too ("...including in the party-balance report (BedBes
//! equivalent)"), and no other roadmap step names it — built here, scoped
//! extension flagged rather than silently skipped, same precedent as 4.5's
//! A9 follow-up. `get_party_balances` reuses `fetch_opening`/`fetch_movement`
//! per party per control-account coordinate (3.1's `coordinate()`/
//! `find_account_id_by_codes`, same attribution 3.2's `Jari_Rem` port
//! already uses) rather than a fourth reimplementation of the windowing
//! logic. `ponytail:` this is O(parties × control-account templates) queries
//! per request — fine at this data scale (a handful of control-account
//! templates per tenant); if this ever needs to scale, replace with one
//! aggregate query joining `parties`/`party_account_config`/`accounts`.
//! **Judgment call**: the legacy's default amount window (`M1 = 1,000,000`,
//! `M2 = 100,000,000`) silently hides most parties out of the box
//! (04-01-a.md §1.2's own "trap" callout) — not reproduced; `minAmount`/
//! `maxAmount` default to unfiltered here, since nothing in the Build
//! bullet asks for the trap to be preserved and no UI exists yet to make the
//! filter's presence visible.

use crate::{
    auth::{authz, AuthUser},
    db,
    parties::{coordinate, fetch_all_config, find_account_id_by_codes, ConfigRow},
    AppState,
};
use axum::{
    extract::{Query, State},
    http::StatusCode,
    routing::get,
    Json, Router,
};
use chrono::NaiveDate;
use serde::Deserialize;
use serde_json::{json, Value};
use sqlx::{Postgres, Transaction};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/ledger", get(get_ledger))
        .route("/party-balances", get(get_party_balances))
}

fn internal_error() -> (StatusCode, Json<Value>) {
    (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": "internal_error" })))
}
fn bad_request(error: &str) -> (StatusCode, Json<Value>) {
    (StatusCode::BAD_REQUEST, Json(json!({ "error": error })))
}
fn forbidden(error: &str) -> (StatusCode, Json<Value>) {
    (StatusCode::FORBIDDEN, Json(json!({ "error": error })))
}

/// Either an exact/partial 4-segment coordinate (a subtree — `moein: None`
/// means "every account under this Kol", matching `DKolU`; every segment
/// given means one exact leaf, matching `DMoein`) or an explicit set of
/// account ids (matching `TMoein`'s caller-supplied predicate). Never both —
/// `ids` wins when present (see `build_filter`).
struct AccountFilter {
    kol: Option<i32>,
    moein: Option<i32>,
    tafsil1: Option<i32>,
    tafsil2: Option<i32>,
    ids: Option<Vec<i64>>,
}

/// The B4 fix: this exact text, and nothing else, is spliced into BOTH
/// `fetch_opening` and `fetch_movement` — there is no second copy of this
/// predicate anywhere in the codebase for the two legs to disagree about.
const ACCOUNT_AND_YEAR_PREDICATE: &str = "\
    ($2::bigint IS NULL OR vl.fiscal_year_id = $2) \
    AND ( \
        ($7::bigint[] IS NOT NULL AND vl.account_id = ANY($7)) \
        OR ($7::bigint[] IS NULL AND a.general_ledger_code = $3 \
            AND ($4::int IS NULL OR a.subsidiary_code = $4) \
            AND ($5::int IS NULL OR a.analytic1_code = $5) \
            AND ($6::int IS NULL OR a.analytic2_code = $6)) \
    )";

/// Net, signed, credit-positive opening balance for `line_date < before_date`
/// (B5: strictly less-than, uniformly).
async fn fetch_opening(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: i64,
    filter: &AccountFilter,
    fiscal_year_id: Option<i64>,
    before_date: NaiveDate,
) -> Result<i64, sqlx::Error> {
    sqlx::query_scalar(&format!(
        "SELECT COALESCE(SUM(vl.credit_amount - vl.debit_amount), 0)::bigint \
         FROM voucher_lines vl JOIN accounts a ON a.id = vl.account_id \
         WHERE vl.tenant_id = $1 AND vl.status = 'posted' AND vl.kind = 'ledger' \
           AND {ACCOUNT_AND_YEAR_PREDICATE} \
           AND vl.line_date < $8"
    ))
    .bind(tenant_id)
    .bind(fiscal_year_id)
    .bind(filter.kol)
    .bind(filter.moein)
    .bind(filter.tafsil1)
    .bind(filter.tafsil2)
    .bind(&filter.ids)
    .bind(before_date)
    .fetch_one(&mut **tx)
    .await
}

#[derive(sqlx::FromRow)]
struct MovementRow {
    id: i64,
    line_date: NaiveDate,
    voucher_id: i64,
    voucher_number: i32,
    description: Option<String>,
    debit_amount: i64,
    credit_amount: i64,
}

/// `[from_date, to_date]` inclusive, ordered `(date, voucher_number, line
/// id)` — 04-03-a.md §3.1.c's tie-break fix.
async fn fetch_movement(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: i64,
    filter: &AccountFilter,
    fiscal_year_id: Option<i64>,
    from_date: NaiveDate,
    to_date: NaiveDate,
) -> Result<Vec<MovementRow>, sqlx::Error> {
    sqlx::query_as(&format!(
        "SELECT vl.id, vl.line_date, vl.voucher_id, v.voucher_number, vl.description, \
                vl.debit_amount, vl.credit_amount \
         FROM voucher_lines vl \
         JOIN accounts a ON a.id = vl.account_id \
         JOIN vouchers v ON v.id = vl.voucher_id \
         WHERE vl.tenant_id = $1 AND vl.status = 'posted' AND vl.kind = 'ledger' \
           AND {ACCOUNT_AND_YEAR_PREDICATE} \
           AND vl.line_date >= $8 AND vl.line_date <= $9 \
         ORDER BY vl.line_date, v.voucher_number, vl.id"
    ))
    .bind(tenant_id)
    .bind(fiscal_year_id)
    .bind(filter.kol)
    .bind(filter.moein)
    .bind(filter.tafsil1)
    .bind(filter.tafsil2)
    .bind(&filter.ids)
    .bind(from_date)
    .bind(to_date)
    .fetch_all(&mut **tx)
    .await
}

/// `Is_Admin_Or_Valid_Daftar`'s real rule: true if ANY segment actually on
/// record for this filter is locked. A missing `accounts` row is never
/// locked (matches the legacy exactly, 04-03-a.md §3.1's permission note).
async fn any_segment_locked(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: i64,
    filter: &AccountFilter,
) -> Result<bool, sqlx::Error> {
    if let Some(ids) = &filter.ids {
        return sqlx::query_scalar(
            "SELECT COALESCE(bool_or(is_locked), false) FROM accounts \
             WHERE tenant_id = $1 AND id = ANY($2)",
        )
        .bind(tenant_id)
        .bind(ids)
        .fetch_one(&mut **tx)
        .await;
    }
    let kol = filter.kol.expect("build_filter always sets kol when ids is None");
    sqlx::query_scalar(
        "SELECT COALESCE(bool_or(is_locked), false) FROM accounts \
         WHERE tenant_id = $1 AND general_ledger_code = $2 AND ( \
           (subsidiary_code = 0 AND analytic1_code = 0 AND analytic2_code = 0) \
           OR ($3::int IS NOT NULL AND subsidiary_code = $3 AND analytic1_code = 0 AND analytic2_code = 0) \
           OR ($3::int IS NOT NULL AND $4::int IS NOT NULL AND subsidiary_code = $3 \
               AND analytic1_code = $4 AND analytic2_code = 0) \
           OR ($3::int IS NOT NULL AND $4::int IS NOT NULL AND $5::int IS NOT NULL AND subsidiary_code = $3 \
               AND analytic1_code = $4 AND analytic2_code = $5) \
         )",
    )
    .bind(tenant_id)
    .bind(kol)
    .bind(filter.moein)
    .bind(filter.tafsil1)
    .bind(filter.tafsil2)
    .fetch_one(&mut **tx)
    .await
}

/// `{ amount, side }` once — not the legacy's signed-grid-vs-`ABS()`-plus-
/// letter print duplication (04-03-a.md §3.1.b).
fn signed(net: i64) -> Value {
    if net >= 0 {
        json!({ "amount": net, "side": "credit" })
    } else {
        json!({ "amount": -net, "side": "debit" })
    }
}

// ---------------------------------------------------------------------
// GET /ledger — DKolU + DMoein + TMoein merged.
// ---------------------------------------------------------------------

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct LedgerQuery {
    fiscal_year_id: Option<i64>,
    from_date: NaiveDate,
    to_date: NaiveDate,
    general_ledger_code: Option<i32>,
    subsidiary_code: Option<i32>,
    analytic1_code: Option<i32>,
    analytic2_code: Option<i32>,
    /// Comma-separated account ids — `TMoein`'s arbitrary caller-supplied
    /// set. Takes precedence over the coordinate fields when present.
    account_ids: Option<String>,
}

fn build_filter(params: &LedgerQuery) -> Result<AccountFilter, (StatusCode, Json<Value>)> {
    if let Some(csv) = &params.account_ids {
        let ids: Result<Vec<i64>, _> = csv.split(',').map(|s| s.trim().parse::<i64>()).collect();
        let ids = ids.map_err(|_| bad_request("invalid_account_ids"))?;
        if ids.is_empty() {
            return Err(bad_request("invalid_account_ids"));
        }
        return Ok(AccountFilter { kol: None, moein: None, tafsil1: None, tafsil2: None, ids: Some(ids) });
    }
    let Some(kol) = params.general_ledger_code else {
        return Err(bad_request("general_ledger_code_required"));
    };
    Ok(AccountFilter {
        kol: Some(kol),
        moein: params.subsidiary_code,
        tafsil1: params.analytic1_code,
        tafsil2: params.analytic2_code,
        ids: None,
    })
}

async fn get_ledger(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(params): Query<LedgerQuery>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    // Merges DKolU (1141, general_ledger) and DMoein (1123,
    // view_subsidiary_ledger) into one route (6.2's own "merge verdict") --
    // either grant unlocks it, same "any one of the ids a merged screen
    // replaces" pattern `vouchers.rs` already uses for its own list/get.
    authz::require_any(&auth, &["view_subsidiary_ledger", "general_ledger"])?;
    if params.from_date > params.to_date {
        return Err(bad_request("date_range_inverted"));
    }
    let filter = build_filter(&params)?;
    let mut tx = db::begin(&state.pool, auth.tenant_id).await.map_err(|_| internal_error())?;

    if !auth.is_superuser {
        let locked = any_segment_locked(&mut tx, auth.tenant_id, &filter).await.map_err(|_| internal_error())?;
        if locked {
            tx.rollback().await.ok();
            // The legacy's own refusal message is wrong about its own rule
            // (04-03-a.md §3.1) -- this one names the actual rule.
            return Err(forbidden("account_segment_locked"));
        }
    }

    let opening = fetch_opening(&mut tx, auth.tenant_id, &filter, params.fiscal_year_id, params.from_date)
        .await
        .map_err(|_| internal_error())?;
    let movement = fetch_movement(&mut tx, auth.tenant_id, &filter, params.fiscal_year_id, params.from_date, params.to_date)
        .await
        .map_err(|_| internal_error())?;
    tx.rollback().await.ok();

    let mut running = opening;
    let rows: Vec<Value> = movement
        .iter()
        .map(|r| {
            running += r.credit_amount - r.debit_amount;
            json!({
                "lineId": r.id,
                "date": r.line_date,
                "voucherId": r.voucher_id,
                "voucherNumber": r.voucher_number,
                "description": r.description,
                "debit": r.debit_amount,
                "credit": r.credit_amount,
                "runningBalance": signed(running),
            })
        })
        .collect();

    Ok(Json(json!({
        "fiscalYearId": params.fiscal_year_id,
        "fromDate": params.from_date,
        "toDate": params.to_date,
        "openingBalance": signed(opening),
        "rows": rows,
        "closingBalance": signed(running),
    })))
}

// ---------------------------------------------------------------------
// GET /party-balances — BedBes / R09 equivalent (B5's other half).
// ---------------------------------------------------------------------

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PartyBalancesQuery {
    fiscal_year_id: Option<i64>,
    from_date: NaiveDate,
    to_date: NaiveDate,
    /// `debtors` | `creditors` — `None` = both, an intentional superset of
    /// the legacy's mandatory 2-item radio (04-01-a.md §1.2).
    side: Option<String>,
    /// `with` | `without` — matches `@GType`; `None` = all (§1.2's `GType=0`).
    turnover: Option<String>,
    min_amount: Option<i64>,
    max_amount: Option<i64>,
}

async fn get_party_balances(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(params): Query<PartyBalancesQuery>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    authz::require(&auth, "debtors_creditors_report")?;
    if params.from_date > params.to_date {
        return Err(bad_request("date_range_inverted"));
    }
    let mut tx = db::begin(&state.pool, auth.tenant_id).await.map_err(|_| internal_error())?;

    let configs: Vec<ConfigRow> = fetch_all_config(&mut tx, auth.tenant_id)
        .await
        .map_err(|_| internal_error())?
        .into_iter()
        .filter(|c| c.counts_toward_balance) // SC_Rem = 1, same subset 3.2's Jari_Rem port uses
        .collect();

    let parties: Vec<(i64, i32, String)> = sqlx::query_as(
        "SELECT id, card_number, first_name || ' ' || last_name FROM parties \
         WHERE tenant_id = $1 ORDER BY card_number",
    )
    .bind(auth.tenant_id)
    .fetch_all(&mut *tx)
    .await
    .map_err(|_| internal_error())?;

    let mut rows = Vec::new();
    let mut grand_period_debit: i64 = 0;
    let mut grand_period_credit: i64 = 0;
    let mut grand_closing: i64 = 0;

    for (party_id, card_number, name) in parties {
        let mut opening_total: i64 = 0;
        let mut period_debit_total: i64 = 0;
        let mut period_credit_total: i64 = 0;
        let mut touched = false;

        for config in &configs {
            let codes = coordinate(config, card_number);
            let Some(account_id) = find_account_id_by_codes(&mut tx, auth.tenant_id, codes)
                .await
                .map_err(|_| internal_error())?
            else {
                continue; // no leaf provisioned at this coordinate -> contributes nothing
            };
            let filter = AccountFilter { kol: None, moein: None, tafsil1: None, tafsil2: None, ids: Some(vec![account_id]) };
            let opening = fetch_opening(&mut tx, auth.tenant_id, &filter, params.fiscal_year_id, params.from_date)
                .await
                .map_err(|_| internal_error())?;
            let movement = fetch_movement(&mut tx, auth.tenant_id, &filter, params.fiscal_year_id, params.from_date, params.to_date)
                .await
                .map_err(|_| internal_error())?;
            opening_total += opening;
            for m in &movement {
                period_debit_total += m.debit_amount;
                period_credit_total += m.credit_amount;
                touched = true;
            }
        }

        // @GType.
        match params.turnover.as_deref() {
            Some("with") if !touched => continue,
            Some("without") if touched => continue,
            _ => {}
        }

        let closing_total = opening_total + period_credit_total - period_debit_total;

        // @BedBes: a sign flip around the amount window, so the window is
        // always expressed in the *natural* direction of the chosen side
        // (04-01-a.md §1.2's "Debtor/creditor selection is a sign flip").
        let selector = match params.side.as_deref() {
            Some("debtors") => -closing_total,
            _ => closing_total,
        };
        if let Some(min) = params.min_amount {
            if selector < min {
                continue;
            }
        }
        if let Some(max) = params.max_amount {
            if selector > max {
                continue;
            }
        }
        if params.side.as_deref() == Some("creditors") && closing_total < 0 {
            continue;
        }
        if params.side.as_deref() == Some("debtors") && closing_total > 0 {
            continue;
        }

        grand_period_debit += period_debit_total;
        grand_period_credit += period_credit_total;
        grand_closing += closing_total;

        rows.push(json!({
            "partyId": party_id,
            "cardNumber": card_number,
            "name": name,
            "openingBalance": signed(opening_total),
            "periodDebit": period_debit_total,
            "periodCredit": period_credit_total,
            "closingBalance": signed(closing_total),
        }));
    }
    tx.rollback().await.ok();

    Ok(Json(json!({
        "fiscalYearId": params.fiscal_year_id,
        "fromDate": params.from_date,
        "toDate": params.to_date,
        "rows": rows,
        "grandTotal": {
            "periodDebit": grand_period_debit,
            "periodCredit": grand_period_credit,
            "closingBalance": signed(grand_closing),
        },
    })))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signed_splits_net_into_amount_and_side_never_negative_amount() {
        assert_eq!(signed(1_000), json!({ "amount": 1000, "side": "credit" }));
        assert_eq!(signed(-1_000), json!({ "amount": 1000, "side": "debit" }));
        assert_eq!(signed(0), json!({ "amount": 0, "side": "credit" }));
    }
}
