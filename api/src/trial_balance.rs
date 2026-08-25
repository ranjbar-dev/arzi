//! Step 6.1 (docs/phase-6-reporting.md §6.1): the 4-column and 6-column
//! trial balances. specs/04-reporting/04-02-a/b-trial-balances-in-depth.md
//! are the behavioural ground truth this module ports — and the closing
//! recommendation of 04-02-b.md §2.3 is followed literally: **one**
//! trial-balance engine (`fetch_level_rows` below) with an explicit
//! `before_date`/`upto_date` window, feeding two thin renderers, rather than
//! the legacy's two architecturally unrelated reports (inline `#temp`-table
//! SQL vs. an opaque stored procedure) that "do not share a single line of
//! code, a single column name, or the same definition of turnover."
//!
//! **A8 applied unconditionally**: every query below filters
//! `status = 'posted'` — no equivalent of the legacy 4-column report's three
//! voucher-state checkboxes that were wired to nothing (04-02-a.md §2.1's
//! defect (a)), and no equivalent of the 6-column report's real-but-partial
//! `@Sabt` bitmask (confirmed and/or posted, never excluding both). A8's
//! ruling ("exclude drafts everywhere") is simpler than either legacy
//! control set and resolves both at once.
//!
//! **Fiscal-year selector actually scopes the query** (04-02-a.md §2.1
//! defect (b) fixed structurally) — `fiscal_year_id` is a required bind on
//! every query below, not a display-only parameter.
//!
//! **The new balance-proof** (04-02-a.md §2.1's "single most important
//! defect to fix"): both endpoints sum debit/credit at Kol level
//! (`fetch_level_rows` with `level = 1`, fetched independently of whatever
//! detail level was requested) and return the two sums plus a `balanced`
//! flag — the caller must decide what to do with a `false`, but the number
//! is always computed and always returned, never silently absorbed into two
//! clamped columns the way the legacy's report is.
//!
//! **Grand total never double-counts** (04-02-a.md §2.1's `IIF(St>1,0,1)`
//! anti-double-count device): `grandTotal` in every response is built from
//! the independently-fetched Kol-level rows, never a sum over the
//! (possibly multi-level, possibly interleaved) `rows` array.
//!
//! **Judgment call, permission wiring deferred**: no dedicated Phase 6
//! wiring step exists yet in the roadmap for these two routes specifically —
//! 6.7 ("Report permission gating") is that step, matching every other
//! domain's established pattern (Phase 3/4/5.1 deferred the same way ahead
//! of their own catalogue ids). Plain `AuthUser` only, for now — a real gap,
//! flagged not hidden, same as those precedents.
//!
//! **Judgment call, no dedicated UI**: unlike every other Phase 2-5 domain,
//! the roadmap's Phase 6 table has no separate "reporting screens" step
//! paired with 6.1-6.4 — 6.5 (print) and 6.6 (export) are the two documented
//! output surfaces for this whole phase. Rather than guess at an
//! interactive screen no roadmap step actually scopes, this step ships the
//! API only, self-verified live via curl/automated tests exactly like every
//! prior schema+API-only step (2.1, 3.1, 4.1, 5.1) was before its own
//! dedicated UI step existed — flagged here since Phase 6 has no such
//! step to point to.

use crate::{auth::authz, auth::AuthUser, db, AppState};
use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::{Postgres, Transaction};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/trial-balance-4-column", get(trial_balance_4_column))
        .route(
            "/trial-balance-4-column/pdf",
            get(trial_balance_4_column_pdf),
        )
        .route("/trial-balance-6-column", get(trial_balance_6_column))
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
fn not_found(what: &str) -> (StatusCode, Json<Value>) {
    (
        StatusCode::NOT_FOUND,
        Json(json!({ "error": format!("{what}_not_found") })),
    )
}

/// 04-02-a.md §2.1's four levels — Kol only, up through all four segments.
fn level_num(level: &str) -> Result<i16, (StatusCode, Json<Value>)> {
    match level {
        "kol" => Ok(1),
        "moein" => Ok(2),
        "tafsil1" => Ok(3),
        "tafsil2" => Ok(4),
        _ => Err(bad_request("invalid_level")),
    }
}

fn default_level() -> String {
    "kol".to_string()
}

/// `max(debit - credit, 0)`, symmetrically — the legacy's exact netting rule
/// (04-02-a.md §2.1 "How Bed/Bes netting works": `RBed - RBes` then clamp
/// both to zero, never a signed figure, never both non-zero at once).
fn net(debit: i64, credit: i64) -> (i64, i64) {
    ((debit - credit).max(0), (credit - debit).max(0))
}

#[derive(sqlx::FromRow, Clone)]
struct RawRow {
    general_ledger_code: i32,
    subsidiary_code: i32,
    analytic1_code: i32,
    analytic2_code: i32,
    name: String,
    debit_before: i64,
    credit_before: i64,
    debit_upto: i64,
    credit_upto: i64,
}

/// The one query every level and every report is built from: gross
/// cumulative debit/credit strictly before `before_date`, and gross
/// cumulative debit/credit up to and including `upto_date`, grouped and
/// rolled up to `level`, `status = 'posted'` always. The group's own display name is looked up from
/// the `accounts` row that IS that level's node (e.g. the Kol row itself,
/// `subsidiary_code = 0`), not synthesised — matching the legacy's own
/// `Update #R Set Name=(Select S_name from sarfasl where ...)` join.
async fn fetch_level_rows(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: i64,
    fiscal_year_id: i64,
    level: i16,
    before_date: NaiveDate,
    upto_date: NaiveDate,
) -> Result<Vec<RawRow>, sqlx::Error> {
    let select_cols = match level {
        1 => {
            "la.general_ledger_code AS general_ledger_code, 0::int AS subsidiary_code, \
              0::int AS analytic1_code, 0::int AS analytic2_code"
        }
        2 => {
            "la.general_ledger_code AS general_ledger_code, la.subsidiary_code AS subsidiary_code, \
              0::int AS analytic1_code, 0::int AS analytic2_code"
        }
        3 => {
            "la.general_ledger_code AS general_ledger_code, la.subsidiary_code AS subsidiary_code, \
              la.analytic1_code AS analytic1_code, 0::int AS analytic2_code"
        }
        4 => {
            "la.general_ledger_code AS general_ledger_code, la.subsidiary_code AS subsidiary_code, \
              la.analytic1_code AS analytic1_code, la.analytic2_code AS analytic2_code"
        }
        _ => unreachable!("level_num only ever returns 1..=4"),
    };
    let group_cols = match level {
        1 => "la.general_ledger_code",
        2 => "la.general_ledger_code, la.subsidiary_code",
        3 => "la.general_ledger_code, la.subsidiary_code, la.analytic1_code",
        4 => "la.general_ledger_code, la.subsidiary_code, la.analytic1_code, la.analytic2_code",
        _ => unreachable!(),
    };
    // 04-02-a.md §2.1: levels 3/4 add "and M_ta1>0"/"and M_ta2>0" so leaf
    // accounts shallower than the requested level don't generate degenerate
    // rows; levels 1/2 have no such filter (every posted leaf always has a
    // non-zero subsidiary_code, per `accounts_segment_hierarchy`).
    let extra_filter = match level {
        3 => " AND la.analytic1_code > 0",
        4 => " AND la.analytic2_code > 0",
        _ => "",
    };
    let ka_join = match level {
        1 => "ka.subsidiary_code = 0 AND ka.analytic1_code = 0 AND ka.analytic2_code = 0",
        2 => "ka.subsidiary_code = la.subsidiary_code AND ka.analytic1_code = 0 AND ka.analytic2_code = 0",
        3 => "ka.subsidiary_code = la.subsidiary_code AND ka.analytic1_code = la.analytic1_code \
              AND ka.analytic2_code = 0",
        4 => "ka.subsidiary_code = la.subsidiary_code AND ka.analytic1_code = la.analytic1_code \
              AND ka.analytic2_code = la.analytic2_code",
        _ => unreachable!(),
    };

    let sql = format!(
        "SELECT {select_cols}, ka.name AS name, \
           COALESCE(SUM(vl.debit_amount)  FILTER (WHERE vl.line_date < $3), 0)::bigint AS debit_before, \
           COALESCE(SUM(vl.credit_amount) FILTER (WHERE vl.line_date < $3), 0)::bigint AS credit_before, \
           COALESCE(SUM(vl.debit_amount)  FILTER (WHERE vl.line_date <= $4), 0)::bigint AS debit_upto, \
           COALESCE(SUM(vl.credit_amount) FILTER (WHERE vl.line_date <= $4), 0)::bigint AS credit_upto \
         FROM voucher_lines vl \
         JOIN accounts la ON la.id = vl.account_id \
         JOIN accounts ka ON ka.tenant_id = la.tenant_id \
              AND ka.general_ledger_code = la.general_ledger_code AND {ka_join} \
         WHERE vl.tenant_id = $1 AND vl.fiscal_year_id = $2 \
           AND vl.status = 'posted' \
           AND vl.line_date <= $4{extra_filter} \
         GROUP BY {group_cols}, ka.name \
         ORDER BY {group_cols}"
    );

    sqlx::query_as(&sql)
        .bind(tenant_id)
        .bind(fiscal_year_id)
        .bind(before_date)
        .bind(upto_date)
        .fetch_all(&mut **tx)
        .await
}

async fn fetch_fiscal_year_start(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: i64,
    fiscal_year_id: i64,
) -> Result<Option<NaiveDate>, sqlx::Error> {
    sqlx::query_scalar("SELECT start_date FROM fiscal_years WHERE tenant_id = $1 AND id = $2")
        .bind(tenant_id)
        .bind(fiscal_year_id)
        .fetch_optional(&mut **tx)
        .await
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct BalanceProof {
    total_debit: i64,
    total_credit: i64,
    balanced: bool,
}

fn balance_proof(kol_rows: &[RawRow]) -> BalanceProof {
    let total_debit: i64 = kol_rows.iter().map(|r| r.debit_upto).sum();
    let total_credit: i64 = kol_rows.iter().map(|r| r.credit_upto).sum();
    BalanceProof {
        total_debit,
        total_credit,
        balanced: total_debit == total_credit,
    }
}

// ---------------------------------------------------------------------
// 4-column: Taraz4Setooni_U — cumulative since fiscal-year start, netted.
// ---------------------------------------------------------------------

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct FourColumnQuery {
    fiscal_year_id: i64,
    as_of_date: NaiveDate,
    #[serde(default = "default_level")]
    level: String,
}

/// Shared by the JSON and PDF renderers (6.5's own "one query, two
/// renderers" split — never a second, drifting copy of this loop). `None`
/// when the fiscal year doesn't exist.
async fn compute_four_column(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: i64,
    fiscal_year_id: i64,
    max_level: i16,
    as_of_date: NaiveDate,
) -> Result<Option<(Vec<(i16, RawRow)>, BalanceProof)>, sqlx::Error> {
    // "Inception" means the start of the selected fiscal year (04-02-a.md
    // §2.1's own clarification of what the legacy's unbounded `M_Date<=@D1`
    // actually means, given `M_COID` scoping) — every line already has
    // `line_date` inside `[start_date, end_date]` (03-04.md's date-range
    // check on create), so `debit_before`/`credit_before` are always zero;
    // computed anyway so this report reuses the identical `fetch_level_rows`
    // the 6-column report calls, per the module doc comment's "one engine".
    let Some(start_date) = fetch_fiscal_year_start(tx, tenant_id, fiscal_year_id).await? else {
        return Ok(None);
    };

    let mut rows = Vec::new();
    let mut kol_rows: Vec<RawRow> = Vec::new();
    for level in 1..=max_level {
        let raw =
            fetch_level_rows(tx, tenant_id, fiscal_year_id, level, start_date, as_of_date).await?;
        if level == 1 {
            kol_rows = raw.clone();
        }
        for r in raw {
            rows.push((level, r));
        }
    }
    Ok(Some((rows, balance_proof(&kol_rows))))
}

async fn trial_balance_4_column(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(params): Query<FourColumnQuery>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    authz::require(&auth, "trial_balance_4_column")?;
    let max_level = level_num(&params.level)?;
    let mut tx = db::begin(&state.pool, auth.tenant_id)
        .await
        .map_err(|_| internal_error())?;

    let Some((raw_rows, proof)) = compute_four_column(
        &mut tx,
        auth.tenant_id,
        params.fiscal_year_id,
        max_level,
        params.as_of_date,
    )
    .await
    .map_err(|_| internal_error())?
    else {
        return Err(not_found("fiscal_year"));
    };
    tx.rollback().await.ok();

    let rows: Vec<Value> = raw_rows
        .iter()
        .map(|(level, r)| {
            let (balance_debit, balance_credit) = net(r.debit_upto, r.credit_upto);
            json!({
                "level": level,
                "generalLedgerCode": r.general_ledger_code,
                "subsidiaryCode": r.subsidiary_code,
                "analytic1Code": r.analytic1_code,
                "analytic2Code": r.analytic2_code,
                "name": r.name,
                "cumulativeDebit": r.debit_upto,
                "cumulativeCredit": r.credit_upto,
                "balanceDebit": balance_debit,
                "balanceCredit": balance_credit,
            })
        })
        .collect();

    let (grand_balance_debit, grand_balance_credit) = net(proof.total_debit, proof.total_credit);

    Ok(Json(json!({
        "fiscalYearId": params.fiscal_year_id,
        "asOfDate": params.as_of_date,
        "level": params.level,
        "rows": rows,
        "grandTotal": {
            "cumulativeDebit": proof.total_debit,
            "cumulativeCredit": proof.total_credit,
            "balanceDebit": grand_balance_debit,
            "balanceCredit": grand_balance_credit,
        },
        "balanceProof": proof,
    })))
}

/// Step 6.5: the PDF twin of `trial_balance_4_column` — same
/// `compute_four_column` call, rendered through the generic tabular-report
/// template instead of JSON. Columns in right-to-left display order,
/// matching `Taraz4Setooni_U`'s own layout (04-02-a.md §2.1's column
/// table): کد | نام | گردش بدهکار | گردش بستانکار | مانده بدهکار |
/// مانده بستانکار. Indentation by level (§2.1's "four spaces per level,
/// cumulative") is applied to the name cell here in Rust, not the template.
async fn trial_balance_4_column_pdf(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(params): Query<FourColumnQuery>,
) -> Result<axum::response::Response, (StatusCode, Json<Value>)> {
    authz::require(&auth, "trial_balance_4_column")?;
    let max_level = level_num(&params.level)?;
    let mut tx = db::begin(&state.pool, auth.tenant_id)
        .await
        .map_err(|_| internal_error())?;

    let Some((raw_rows, proof)) = compute_four_column(
        &mut tx,
        auth.tenant_id,
        params.fiscal_year_id,
        max_level,
        params.as_of_date,
    )
    .await
    .map_err(|_| internal_error())?
    else {
        return Err(not_found("fiscal_year"));
    };

    let organization_name = crate::pdf::fetch_organization_name(&mut tx, auth.tenant_id)
        .await
        .map_err(|_| internal_error())?;
    let fiscal_year_label: Option<i32> =
        sqlx::query_scalar("SELECT year FROM fiscal_years WHERE tenant_id = $1 AND id = $2")
            .bind(auth.tenant_id)
            .bind(params.fiscal_year_id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(|_| internal_error())?;
    // Taraz4Setooni_U.pas:174's own signature source -- 1014, "report_signature" here.
    let signature_labels =
        crate::pdf::fetch_signature_labels(&mut tx, auth.tenant_id, &["report_signature"])
            .await
            .map_err(|_| internal_error())?
            .into_iter()
            .filter(|s| !s.is_empty())
            .collect();
    tx.rollback().await.ok();

    let (grand_balance_debit, grand_balance_credit) = net(proof.total_debit, proof.total_credit);
    let header = crate::pdf::PrintHeader {
        organization_name,
        fiscal_year_caption: fiscal_year_label
            .map(|y| format!("سال مالی {y}"))
            .unwrap_or_default(),
        report_title: "تراز آزمایشی ۴ ستونی".to_string(),
        period_caption: Some(format!("تا تاریخ: {}", params.as_of_date)),
        // The grand total's NET balance is zero by construction whenever the
        // trial balance actually balances (that is what "balanced" means --
        // Σdebit = Σcredit at Kol level nets to 0), so the netted figure is
        // never the meaningful number to spell out. `proof.total_debit`
        // (== `total_credit` when balanced) is the real headline total the
        // manual test means by "a report with a numeric amount."
        amount_in_words: Some(crate::persian_words::amount_in_words(proof.total_debit)),
        signature_labels,
    };
    let columns = vec![
        "کد".to_string(),
        "نام".to_string(),
        "گردش بدهکار".to_string(),
        "گردش بستانکار".to_string(),
        "مانده بدهکار".to_string(),
        "مانده بستانکار".to_string(),
    ];
    let rows: Vec<Vec<String>> = raw_rows
        .iter()
        .map(|(level, r)| {
            let (balance_debit, balance_credit) = net(r.debit_upto, r.credit_upto);
            let indent = "    ".repeat((*level as usize).saturating_sub(1));
            let code = match level {
                1 => r.general_ledger_code.to_string(),
                2 => format!("{}-{}", r.general_ledger_code, r.subsidiary_code),
                3 => format!(
                    "{}-{}-{}",
                    r.general_ledger_code, r.subsidiary_code, r.analytic1_code
                ),
                _ => format!(
                    "{}-{}-{}-{}",
                    r.general_ledger_code, r.subsidiary_code, r.analytic1_code, r.analytic2_code
                ),
            };
            vec![
                code,
                format!("{indent}{}", r.name),
                crate::pdf::format_amount_public(r.debit_upto),
                crate::pdf::format_amount_public(r.credit_upto),
                crate::pdf::format_amount_public(balance_debit),
                crate::pdf::format_amount_public(balance_credit),
            ]
        })
        .collect();
    let totals = vec![
        String::new(),
        "جمع کل".to_string(),
        crate::pdf::format_amount_public(proof.total_debit),
        crate::pdf::format_amount_public(proof.total_credit),
        crate::pdf::format_amount_public(grand_balance_debit),
        crate::pdf::format_amount_public(grand_balance_credit),
    ];

    let pdf = crate::pdf::render_tabular_report_pdf(header, columns, rows, Some(totals))
        .map_err(|_| internal_error())?;

    Ok((
        [
            (
                axum::http::header::CONTENT_TYPE,
                "application/pdf".to_string(),
            ),
            (
                axum::http::header::CONTENT_DISPOSITION,
                "inline; filename=\"trial-balance-4-column.pdf\"".to_string(),
            ),
        ],
        pdf,
    )
        .into_response())
}

// ---------------------------------------------------------------------
// 6-column: Taraz6SetooniU — opening turnover + period turnover + closing
// balance, single requested level only (04-02-b.md §2.2's "Level selection:
// single @Level value", NOT the 4-column report's cumulative interleave).
// ---------------------------------------------------------------------

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SixColumnQuery {
    fiscal_year_id: i64,
    from_date: NaiveDate,
    to_date: NaiveDate,
    #[serde(default = "default_level")]
    level: String,
}

async fn trial_balance_6_column(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(params): Query<SixColumnQuery>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    authz::require(&auth, "trial_balance_6_column")?;
    let level = level_num(&params.level)?;
    // Fixes 04-02-b.md §2.2's validation bug (2): the legacy sets
    // `ActiveControl := D1` on an inverted range but never `Exit`s, so the
    // query runs anyway and silently returns an empty period. Reject
    // outright instead.
    if params.from_date > params.to_date {
        return Err(bad_request("date_range_inverted"));
    }
    let mut tx = db::begin(&state.pool, auth.tenant_id)
        .await
        .map_err(|_| internal_error())?;

    if fetch_fiscal_year_start(&mut tx, auth.tenant_id, params.fiscal_year_id)
        .await
        .map_err(|_| internal_error())?
        .is_none()
    {
        return Err(not_found("fiscal_year"));
    }

    let raw = fetch_level_rows(
        &mut tx,
        auth.tenant_id,
        params.fiscal_year_id,
        level,
        params.from_date,
        params.to_date,
    )
    .await
    .map_err(|_| internal_error())?;
    let kol_rows = if level == 1 {
        raw.clone()
    } else {
        fetch_level_rows(
            &mut tx,
            auth.tenant_id,
            params.fiscal_year_id,
            1,
            params.from_date,
            params.to_date,
        )
        .await
        .map_err(|_| internal_error())?
    };
    tx.rollback().await.ok();

    let rows: Vec<Value> = raw
        .iter()
        .map(|r| {
            let (closing_debit, closing_credit) = net(r.debit_upto, r.credit_upto);
            json!({
                "level": level,
                "generalLedgerCode": r.general_ledger_code,
                "subsidiaryCode": r.subsidiary_code,
                "analytic1Code": r.analytic1_code,
                "analytic2Code": r.analytic2_code,
                "name": r.name,
                "openingDebit": r.debit_before,
                "openingCredit": r.credit_before,
                "periodDebit": r.debit_upto - r.debit_before,
                "periodCredit": r.credit_upto - r.credit_before,
                "closingDebit": closing_debit,
                "closingCredit": closing_credit,
            })
        })
        .collect();

    let proof = balance_proof(&kol_rows);
    let opening_total_debit: i64 = kol_rows.iter().map(|r| r.debit_before).sum();
    let opening_total_credit: i64 = kol_rows.iter().map(|r| r.credit_before).sum();
    let (grand_closing_debit, grand_closing_credit) = net(proof.total_debit, proof.total_credit);

    Ok(Json(json!({
        "fiscalYearId": params.fiscal_year_id,
        "fromDate": params.from_date,
        "toDate": params.to_date,
        "level": params.level,
        "rows": rows,
        "grandTotal": {
            "openingDebit": opening_total_debit,
            "openingCredit": opening_total_credit,
            "periodDebit": proof.total_debit - opening_total_debit,
            "periodCredit": proof.total_credit - opening_total_credit,
            "closingDebit": grand_closing_debit,
            "closingCredit": grand_closing_credit,
        },
        "balanceProof": {
            "totalDebit": proof.total_debit,
            "totalCredit": proof.total_credit,
            "balanced": proof.balanced,
            "openingBalanced": opening_total_debit == opening_total_credit,
        },
    })))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn net_clamps_negative_side_to_zero_never_both() {
        assert_eq!(net(100, 40), (60, 0));
        assert_eq!(net(40, 100), (0, 60));
        assert_eq!(net(50, 50), (0, 0));
        assert_eq!(net(0, 0), (0, 0));
    }
}
