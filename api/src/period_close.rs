//! Step 2.7 (docs/phase-2-accounting-core.md §2.7): period close / year-end.
//! specs/03-accounting-core/03-09-a/b/c-period-close-and-year-end.md, A7 in
//! specs/11-open-decisions.md.
//!
//! Two operations, both superuser-only (03-09-a.md's opening note — both
//! `NewFinalu` and `EnteghalU` require Supervisor rights):
//!
//! - `close_books` (`NewFinalu` equivalent): zero a chosen set of Kol
//!   (general-ledger) accounts' net balance into one destination account,
//!   two lines per underlying leaf account so the voucher balances by
//!   construction (03-09-a.md §9.2 steps 4-5).
//! - `carry_forward` (`EnteghalU` equivalent): reverse every leaf account's
//!   net balance in the outgoing year into a closing-contra account, and
//!   re-establish it in the incoming year from an opening-contra account
//!   (03-09-b.md §9.3).
//!
//! **A7's enforced order**: `carry_forward` is rejected unless `close_books`
//! has already run for this fiscal year AND that voucher has reached
//! `posted` — tracked via `fiscal_years.books_closed_voucher_id` (0008
//! migration), not inferred from account balances (an already-zeroed P&L
//! account and one that was simply never touched all year look identical
//! from a balance query alone).
//!
//! `FinalU` (the single-Kol dead form) is dropped entirely (C5) but two of
//! its validations that `NewFinalu` genuinely lost are ported per 03-09-c.md:
//! destination must be a leaf account, and destination Kol must not equal a
//! source Kol ("an account cannot be closed to itself") — checked by integer
//! id/code comparison, not the legacy's sentinel-padded string containment
//! test that silently fails open when the destination code has no dash
//! (03-09-a.md §9.2 validation 7).
//!
//! **Judgment call**: the legacy's "voucher number already exists -> append
//! to it" flow (validations 2-4 in both forms) is not ported — every call
//! here allocates or creates a fresh voucher, the same simplification 2.5/
//! 2.6 already made; the incremental per-line editor (2.3/2.4) already lets
//! a user manually consolidate vouchers afterwards if they want to. Lines
//! stay `source_module = 0` (manual/editable) — matching the legacy's
//! "nothing is locked afterwards", same choice as 2.6's journal lines.
//!
//! **Judgment call**: leaf accounts with a net balance that clamps to exactly
//! zero are skipped (no lines emitted) rather than ported verbatim — the
//! legacy's SQL Server had no constraint against a zero/zero row, this
//! schema's `voucher_lines_not_both_zero` check does.
//!
//! **Judgment call**: `carry_forward`'s four-insert sequence runs inside one
//! transaction for the whole operation, not the legacy's one-transaction-
//! per-account loop (`EnteghalU.pas:239-282`) — a strictly safer, free
//! improvement (the legacy hazard the spec calls out, mutating a global
//! "current year" between the two inserts, can't even arise here since the
//! year is always passed explicitly).

use crate::{audit, auth::authz::RequireSuperuser, db, AppState};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use chrono::NaiveDate;
use serde::Deserialize;
use serde_json::{json, Value};
use sqlx::{Postgres, Transaction};
use std::collections::HashSet;

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
        if db_err.constraint() == Some("vouchers_number_key") {
            return (
                StatusCode::CONFLICT,
                Json(json!({ "error": "duplicate_voucher_number" })),
            );
        }
    }
    internal_error()
}

struct FiscalYearRow {
    id: i64,
    year: i32,
    start_date: NaiveDate,
    end_date: NaiveDate,
    is_active: bool,
    books_closed_voucher_id: Option<i64>,
}

async fn fetch_fiscal_year_by_id(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: i64,
    id: i64,
) -> Result<Option<FiscalYearRow>, sqlx::Error> {
    sqlx::query_as(
        "SELECT id, year, start_date, end_date, is_active, books_closed_voucher_id \
         FROM fiscal_years WHERE tenant_id = $1 AND id = $2",
    )
    .bind(tenant_id)
    .bind(id)
    .fetch_optional(&mut **tx)
    .await
    .map(
        |row: Option<(i64, i32, NaiveDate, NaiveDate, bool, Option<i64>)>| {
            row.map(
                |(id, year, start_date, end_date, is_active, books_closed_voucher_id)| {
                    FiscalYearRow {
                        id,
                        year,
                        start_date,
                        end_date,
                        is_active,
                        books_closed_voucher_id,
                    }
                },
            )
        },
    )
}

async fn fetch_fiscal_year_by_year(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: i64,
    year: i32,
) -> Result<Option<FiscalYearRow>, sqlx::Error> {
    sqlx::query_as(
        "SELECT id, year, start_date, end_date, is_active, books_closed_voucher_id \
         FROM fiscal_years WHERE tenant_id = $1 AND year = $2",
    )
    .bind(tenant_id)
    .bind(year)
    .fetch_optional(&mut **tx)
    .await
    .map(
        |row: Option<(i64, i32, NaiveDate, NaiveDate, bool, Option<i64>)>| {
            row.map(
                |(id, year, start_date, end_date, is_active, books_closed_voucher_id)| {
                    FiscalYearRow {
                        id,
                        year,
                        start_date,
                        end_date,
                        is_active,
                        books_closed_voucher_id,
                    }
                },
            )
        },
    )
}

/// (general_ledger_code, subsidiary_code, analytic1_code, analytic2_code, child_count)
async fn fetch_account_tuple(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: i64,
    id: i64,
) -> Result<Option<(i32, i32, i32, i32, i32)>, sqlx::Error> {
    sqlx::query_as(
        "SELECT general_ledger_code, subsidiary_code, analytic1_code, analytic2_code, child_count \
         FROM accounts WHERE tenant_id = $1 AND id = $2",
    )
    .bind(tenant_id)
    .bind(id)
    .fetch_optional(&mut **tx)
    .await
}

// ---------------------------------------------------------------------
// close-books  <-  NewFinalu
// ---------------------------------------------------------------------

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CloseBooksRequest {
    /// Kol (general-ledger, level-1) account ids ticked for closing —
    /// 03-09-a.md §9.2 step 1's candidate grid, selection happens client-side.
    source_kol_account_ids: Vec<i64>,
    destination_account_id: i64,
    voucher_number: Option<i32>,
    voucher_date: NaiveDate,
    description: String,
}

pub async fn close_books(
    State(state): State<AppState>,
    admin: RequireSuperuser,
    Path(fiscal_year_id): Path<i64>,
    Json(req): Json<CloseBooksRequest>,
) -> Result<(StatusCode, Json<Value>), (StatusCode, Json<Value>)> {
    let auth = admin.0;
    if req.description.trim().is_empty() {
        return Err(bad_request("description_required")); // validation #8
    }
    if req.source_kol_account_ids.is_empty() {
        return Err(bad_request("no_source_accounts_selected")); // validation #5
    }

    let mut tx = db::begin(&state.pool, auth.tenant_id)
        .await
        .map_err(|_| internal_error())?;

    let Some(fy) = fetch_fiscal_year_by_id(&mut tx, auth.tenant_id, fiscal_year_id)
        .await
        .map_err(|_| internal_error())?
    else {
        return Err(not_found("fiscal_year"));
    };
    if !fy.is_active {
        return Err(forbidden("fiscal_year_closed"));
    }
    if req.voucher_date < fy.start_date || req.voucher_date > fy.end_date {
        return Err(bad_request("date_outside_fiscal_year"));
    }

    // destination must resolve, and (03-09-c.md's ported FinalU check) must be a leaf account.
    let Some((dest_kol, _, _, _, dest_children)) =
        fetch_account_tuple(&mut tx, auth.tenant_id, req.destination_account_id)
            .await
            .map_err(|_| internal_error())?
    else {
        return Err(bad_request("destination_account_not_found")); // validation #6
    };
    if dest_children > 0 {
        return Err(bad_request("destination_not_leaf"));
    }

    // every ticked id must resolve to a real Kol (level-1) account.
    let mut source_kol_codes: HashSet<i32> = HashSet::new();
    for &id in &req.source_kol_account_ids {
        let Some((kol_code, subsidiary, ta1, ta2, _)) =
            fetch_account_tuple(&mut tx, auth.tenant_id, id)
                .await
                .map_err(|_| internal_error())?
        else {
            return Err(bad_request("source_account_not_found"));
        };
        if subsidiary != 0 || ta1 != 0 || ta2 != 0 {
            return Err(bad_request("source_not_kol_account"));
        }
        source_kol_codes.insert(kol_code);
    }
    // validation #7, fixed: integer comparison, not a sentinel-padded string containment test
    // (03-09-a.md §9.2's "silently fails open when the destination code has no dash"). Also
    // covers FinalU's lost "an account cannot be closed to itself" check at Kol granularity.
    if source_kol_codes.contains(&dest_kol) {
        return Err(bad_request("destination_in_source"));
    }

    // Step 4: roll up every leaf account under the ticked Kols to its net balance
    // (kind = 'ledger' only — matches the legacy's `M_Kind=1` filter, excluding
    // already-summarised daybook/journal lines from double-counting).
    let leaf_balances: Vec<(i64, i64)> = sqlx::query_as(
        "SELECT vl.account_id, SUM(vl.debit_amount - vl.credit_amount)::bigint \
         FROM voucher_lines vl JOIN accounts a ON a.id = vl.account_id \
         WHERE vl.tenant_id = $1 AND vl.fiscal_year_id = $2 AND vl.kind = 'ledger' \
           AND a.general_ledger_code = ANY($3) \
         GROUP BY vl.account_id",
    )
    .bind(auth.tenant_id)
    .bind(fiscal_year_id)
    .bind(source_kol_codes.iter().copied().collect::<Vec<_>>())
    .fetch_all(&mut *tx)
    .await
    .map_err(|_| internal_error())?;

    struct ClosePair {
        account_id: i64,
        net_debit: i64,
        net_credit: i64,
    }
    let pairs: Vec<ClosePair> = leaf_balances
        .into_iter()
        .filter_map(|(account_id, net)| match net {
            n if n > 0 => Some(ClosePair {
                account_id,
                net_debit: n,
                net_credit: 0,
            }),
            n if n < 0 => Some(ClosePair {
                account_id,
                net_debit: 0,
                net_credit: -n,
            }),
            _ => None, // clamps to zero — nothing to close for this leaf
        })
        .collect();
    if pairs.is_empty() {
        return Err(bad_request("no_balance_to_close"));
    }

    let total: i64 = pairs.iter().map(|p| p.net_debit + p.net_credit).sum();

    let voucher_number = match req.voucher_number {
        Some(n) if n > 0 => n,
        Some(_) => return Err(bad_request("invalid_voucher_number")),
        None => sqlx::query_scalar(
            "UPDATE fiscal_years SET next_voucher_number = next_voucher_number + 1 \
             WHERE id = $1 RETURNING next_voucher_number - 1",
        )
        .bind(fiscal_year_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(|_| internal_error())?,
    };

    let voucher_id: i64 = sqlx::query_scalar(
        "INSERT INTO vouchers \
         (tenant_id, fiscal_year_id, voucher_number, voucher_date, description, \
          total_debit, total_credit, line_count, kind, created_by) \
         VALUES ($1, $2, $3, $4, $5, $6, $6, $7, 'ledger', $8) RETURNING id",
    )
    .bind(auth.tenant_id)
    .bind(fiscal_year_id)
    .bind(voucher_number)
    .bind(req.voucher_date)
    .bind(req.description.trim())
    .bind(total)
    .bind((pairs.len() * 2) as i32)
    .bind(auth.user_id)
    .fetch_one(&mut *tx)
    .await
    .map_err(conflict_or_internal)?;

    // Step 5's pairing: source has a net DEBIT -> debit destination / credit source;
    // net CREDIT -> debit source / credit destination. Either way the pair balances.
    for pair in &pairs {
        let (source_debit, source_credit, dest_debit, dest_credit) = if pair.net_debit > 0 {
            (0i64, pair.net_debit, pair.net_debit, 0i64)
        } else {
            (pair.net_credit, 0i64, 0i64, pair.net_credit)
        };
        sqlx::query(
            "INSERT INTO voucher_lines \
             (tenant_id, voucher_id, fiscal_year_id, line_date, debit_amount, credit_amount, \
              description, account_id, kind, created_by) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, 'ledger', $9)",
        )
        .bind(auth.tenant_id)
        .bind(voucher_id)
        .bind(fiscal_year_id)
        .bind(req.voucher_date)
        .bind(source_debit)
        .bind(source_credit)
        .bind(req.description.trim())
        .bind(pair.account_id)
        .bind(auth.user_id)
        .execute(&mut *tx)
        .await
        .map_err(|_| internal_error())?;

        sqlx::query(
            "INSERT INTO voucher_lines \
             (tenant_id, voucher_id, fiscal_year_id, line_date, debit_amount, credit_amount, \
              description, account_id, kind, created_by) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, 'ledger', $9)",
        )
        .bind(auth.tenant_id)
        .bind(voucher_id)
        .bind(fiscal_year_id)
        .bind(req.voucher_date)
        .bind(dest_debit)
        .bind(dest_credit)
        .bind(req.description.trim())
        .bind(req.destination_account_id)
        .bind(auth.user_id)
        .execute(&mut *tx)
        .await
        .map_err(|_| internal_error())?;
    }

    sqlx::query("UPDATE fiscal_years SET books_closed_voucher_id = $1, updated_at = now(), updated_by = $2 WHERE id = $3")
        .bind(voucher_id)
        .bind(auth.user_id)
        .bind(fiscal_year_id)
        .execute(&mut *tx)
        .await
        .map_err(|_| internal_error())?;

    audit::record_mutation(
        &mut tx,
        auth.tenant_id,
        "vouchers",
        voucher_id,
        "insert",
        Some(auth.user_id),
        None,
        Some(json!({
            "purpose": "close_books",
            "voucherNumber": voucher_number,
            "destinationAccountId": req.destination_account_id,
            "sourceKolAccountIds": req.source_kol_account_ids,
        })),
    )
    .await
    .map_err(|_| internal_error())?;
    audit::record_mutation(
        &mut tx,
        auth.tenant_id,
        "fiscal_years",
        fiscal_year_id,
        "update",
        Some(auth.user_id),
        Some(json!({ "booksClosedVoucherId": fy.books_closed_voucher_id })),
        Some(json!({ "booksClosedVoucherId": voucher_id })),
    )
    .await
    .map_err(|_| internal_error())?;

    tx.commit().await.map_err(|_| internal_error())?;
    Ok((StatusCode::CREATED, Json(json!({ "id": voucher_id }))))
}

// ---------------------------------------------------------------------
// carry-forward  <-  EnteghalU
// ---------------------------------------------------------------------

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CarryForwardRequest {
    closing_voucher_number: Option<i32>,
    opening_voucher_number: Option<i32>,
    closing_date: NaiveDate,
    opening_date: NaiveDate,
    closing_description: String,
    opening_description: String,
    /// کد اختتامیه — the contra account in the outgoing year.
    closing_contra_account_id: i64,
    /// کد افتتاحیه — the contra account in the incoming year.
    opening_contra_account_id: i64,
}

pub async fn carry_forward(
    State(state): State<AppState>,
    admin: RequireSuperuser,
    Path(fiscal_year_id): Path<i64>,
    Json(req): Json<CarryForwardRequest>,
) -> Result<(StatusCode, Json<Value>), (StatusCode, Json<Value>)> {
    let auth = admin.0;
    if req.closing_description.trim().is_empty() {
        return Err(bad_request("closing_description_required")); // #11
    }
    if req.opening_description.trim().is_empty() {
        return Err(bad_request("opening_description_required")); // #12
    }

    let mut tx = db::begin(&state.pool, auth.tenant_id)
        .await
        .map_err(|_| internal_error())?;

    let Some(outgoing) = fetch_fiscal_year_by_id(&mut tx, auth.tenant_id, fiscal_year_id)
        .await
        .map_err(|_| internal_error())?
    else {
        return Err(not_found("fiscal_year"));
    };
    let Some(incoming) = fetch_fiscal_year_by_year(&mut tx, auth.tenant_id, outgoing.year + 1)
        .await
        .map_err(|_| internal_error())?
    else {
        return Err(bad_request("next_fiscal_year_missing")); // #1
    };
    if !outgoing.is_active {
        return Err(forbidden("fiscal_year_closed"));
    }
    if !incoming.is_active {
        return Err(forbidden("fiscal_year_closed"));
    }

    // A7: close_books must have run for the outgoing year, and that voucher must be posted.
    let books_closed = match outgoing.books_closed_voucher_id {
        Some(voucher_id) => {
            let status: Option<String> =
                sqlx::query_scalar("SELECT status::text FROM vouchers WHERE id = $1")
                    .bind(voucher_id)
                    .fetch_optional(&mut *tx)
                    .await
                    .map_err(|_| internal_error())?;
            status.as_deref() == Some("posted")
        }
        None => false,
    };
    if !books_closed {
        return Err(bad_request("books_not_closed")); // A7's enforcement point
    }

    // #2: every voucher (any kind) in the outgoing year must be finalised (M_Tx >= 2).
    let has_unposted: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM vouchers WHERE tenant_id = $1 AND fiscal_year_id = $2 \
         AND status <> 'posted')",
    )
    .bind(auth.tenant_id)
    .bind(fiscal_year_id)
    .fetch_one(&mut *tx)
    .await
    .map_err(|_| internal_error())?;
    if has_unposted {
        return Err(bad_request("vouchers_not_all_posted"));
    }

    if req.closing_date < outgoing.start_date || req.closing_date > outgoing.end_date {
        return Err(bad_request("closing_date_outside_fiscal_year")); // #8
    }
    if req.opening_date < incoming.start_date || req.opening_date > incoming.end_date {
        return Err(bad_request("opening_date_outside_fiscal_year")); // #10
    }

    let Some((_, _, _, _, closing_contra_children)) =
        fetch_account_tuple(&mut tx, auth.tenant_id, req.closing_contra_account_id)
            .await
            .map_err(|_| internal_error())?
    else {
        return Err(bad_request("closing_contra_account_not_found")); // #13
    };
    if closing_contra_children > 0 {
        return Err(bad_request("closing_contra_not_leaf"));
    }
    let Some((_, _, _, _, opening_contra_children)) =
        fetch_account_tuple(&mut tx, auth.tenant_id, req.opening_contra_account_id)
            .await
            .map_err(|_| internal_error())?
    else {
        return Err(bad_request("opening_contra_account_not_found")); // #14
    };
    if opening_contra_children > 0 {
        return Err(bad_request("opening_contra_not_leaf"));
    }

    // Driving query (03-09-b.md §9.3): every leaf account of the outgoing year with a
    // nonzero net balance, kind = 'ledger' only (matches the legacy's `M_kind=1` filter).
    let leaf_balances: Vec<(i64, i64)> = sqlx::query_as(
        "SELECT account_id, SUM(debit_amount - credit_amount)::bigint FROM voucher_lines \
         WHERE tenant_id = $1 AND fiscal_year_id = $2 AND kind = 'ledger' \
         GROUP BY account_id",
    )
    .bind(auth.tenant_id)
    .bind(fiscal_year_id)
    .fetch_all(&mut *tx)
    .await
    .map_err(|_| internal_error())?;

    struct CarryPair {
        account_id: i64,
        net_debit: i64,
        net_credit: i64,
    }
    let pairs: Vec<CarryPair> = leaf_balances
        .into_iter()
        .filter_map(|(account_id, net)| match net {
            n if n > 0 => Some(CarryPair {
                account_id,
                net_debit: n,
                net_credit: 0,
            }),
            n if n < 0 => Some(CarryPair {
                account_id,
                net_debit: 0,
                net_credit: -n,
            }),
            _ => None,
        })
        .collect();
    if pairs.is_empty() {
        return Err(bad_request("already_carried_forward")); // #15
    }

    let total: i64 = pairs.iter().map(|p| p.net_debit + p.net_credit).sum();

    let closing_number = match req.closing_voucher_number {
        Some(n) if n > 0 => n,
        Some(_) => return Err(bad_request("invalid_closing_voucher_number")),
        None => sqlx::query_scalar(
            "UPDATE fiscal_years SET next_voucher_number = next_voucher_number + 1 \
             WHERE id = $1 RETURNING next_voucher_number - 1",
        )
        .bind(fiscal_year_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(|_| internal_error())?,
    };
    let opening_number = match req.opening_voucher_number {
        Some(n) if n > 0 => n,
        Some(_) => return Err(bad_request("invalid_opening_voucher_number")),
        None => sqlx::query_scalar(
            "UPDATE fiscal_years SET next_voucher_number = next_voucher_number + 1 \
             WHERE id = $1 RETURNING next_voucher_number - 1",
        )
        .bind(incoming.id)
        .fetch_one(&mut *tx)
        .await
        .map_err(|_| internal_error())?,
    };

    let closing_voucher_id: i64 = sqlx::query_scalar(
        "INSERT INTO vouchers \
         (tenant_id, fiscal_year_id, voucher_number, voucher_date, description, \
          total_debit, total_credit, line_count, kind, created_by) \
         VALUES ($1, $2, $3, $4, $5, $6, $6, $7, 'ledger', $8) RETURNING id",
    )
    .bind(auth.tenant_id)
    .bind(fiscal_year_id)
    .bind(closing_number)
    .bind(req.closing_date)
    .bind(req.closing_description.trim())
    .bind(total)
    .bind((pairs.len() * 2) as i32)
    .bind(auth.user_id)
    .fetch_one(&mut *tx)
    .await
    .map_err(conflict_or_internal)?;

    let opening_voucher_id: i64 = sqlx::query_scalar(
        "INSERT INTO vouchers \
         (tenant_id, fiscal_year_id, voucher_number, voucher_date, description, \
          total_debit, total_credit, line_count, kind, created_by) \
         VALUES ($1, $2, $3, $4, $5, $6, $6, $7, 'ledger', $8) RETURNING id",
    )
    .bind(auth.tenant_id)
    .bind(incoming.id)
    .bind(opening_number)
    .bind(req.opening_date)
    .bind(req.opening_description.trim())
    .bind(total)
    .bind((pairs.len() * 2) as i32)
    .bind(auth.user_id)
    .fetch_one(&mut *tx)
    .await
    .map_err(conflict_or_internal)?;

    for pair in &pairs {
        // (1) reverse A to zero in the outgoing year's closing voucher.
        sqlx::query(
            "INSERT INTO voucher_lines \
             (tenant_id, voucher_id, fiscal_year_id, line_date, debit_amount, credit_amount, \
              description, account_id, kind, created_by) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, 'ledger', $9)",
        )
        .bind(auth.tenant_id)
        .bind(closing_voucher_id)
        .bind(fiscal_year_id)
        .bind(req.closing_date)
        .bind(pair.net_credit)
        .bind(pair.net_debit)
        .bind(req.closing_description.trim())
        .bind(pair.account_id)
        .bind(auth.user_id)
        .execute(&mut *tx)
        .await
        .map_err(|_| internal_error())?;
        // (2) absorb the balance into the closing-contra account.
        sqlx::query(
            "INSERT INTO voucher_lines \
             (tenant_id, voucher_id, fiscal_year_id, line_date, debit_amount, credit_amount, \
              description, account_id, kind, created_by) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, 'ledger', $9)",
        )
        .bind(auth.tenant_id)
        .bind(closing_voucher_id)
        .bind(fiscal_year_id)
        .bind(req.closing_date)
        .bind(pair.net_debit)
        .bind(pair.net_credit)
        .bind(req.closing_description.trim())
        .bind(req.closing_contra_account_id)
        .bind(auth.user_id)
        .execute(&mut *tx)
        .await
        .map_err(|_| internal_error())?;
        // (3) re-establish A in the incoming year's opening voucher.
        sqlx::query(
            "INSERT INTO voucher_lines \
             (tenant_id, voucher_id, fiscal_year_id, line_date, debit_amount, credit_amount, \
              description, account_id, kind, created_by) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, 'ledger', $9)",
        )
        .bind(auth.tenant_id)
        .bind(opening_voucher_id)
        .bind(incoming.id)
        .bind(req.opening_date)
        .bind(pair.net_debit)
        .bind(pair.net_credit)
        .bind(req.opening_description.trim())
        .bind(pair.account_id)
        .bind(auth.user_id)
        .execute(&mut *tx)
        .await
        .map_err(|_| internal_error())?;
        // (4) contra in the incoming year.
        sqlx::query(
            "INSERT INTO voucher_lines \
             (tenant_id, voucher_id, fiscal_year_id, line_date, debit_amount, credit_amount, \
              description, account_id, kind, created_by) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, 'ledger', $9)",
        )
        .bind(auth.tenant_id)
        .bind(opening_voucher_id)
        .bind(incoming.id)
        .bind(req.opening_date)
        .bind(pair.net_credit)
        .bind(pair.net_debit)
        .bind(req.opening_description.trim())
        .bind(req.opening_contra_account_id)
        .bind(auth.user_id)
        .execute(&mut *tx)
        .await
        .map_err(|_| internal_error())?;
    }

    sqlx::query(
        "UPDATE fiscal_years SET closing_voucher_id = $1, updated_at = now(), updated_by = $2 WHERE id = $3",
    )
    .bind(closing_voucher_id)
    .bind(auth.user_id)
    .bind(fiscal_year_id)
    .execute(&mut *tx)
    .await
    .map_err(|_| internal_error())?;
    sqlx::query(
        "UPDATE fiscal_years SET opening_voucher_id = $1, updated_at = now(), updated_by = $2 WHERE id = $3",
    )
    .bind(opening_voucher_id)
    .bind(auth.user_id)
    .bind(incoming.id)
    .execute(&mut *tx)
    .await
    .map_err(|_| internal_error())?;

    audit::record_mutation(
        &mut tx,
        auth.tenant_id,
        "vouchers",
        closing_voucher_id,
        "insert",
        Some(auth.user_id),
        None,
        Some(json!({ "purpose": "carry_forward_closing", "voucherNumber": closing_number })),
    )
    .await
    .map_err(|_| internal_error())?;
    audit::record_mutation(
        &mut tx,
        auth.tenant_id,
        "vouchers",
        opening_voucher_id,
        "insert",
        Some(auth.user_id),
        None,
        Some(json!({ "purpose": "carry_forward_opening", "voucherNumber": opening_number })),
    )
    .await
    .map_err(|_| internal_error())?;
    audit::record_mutation(
        &mut tx,
        auth.tenant_id,
        "fiscal_years",
        fiscal_year_id,
        "update",
        Some(auth.user_id),
        None,
        Some(json!({ "closingVoucherId": closing_voucher_id })),
    )
    .await
    .map_err(|_| internal_error())?;
    audit::record_mutation(
        &mut tx,
        auth.tenant_id,
        "fiscal_years",
        incoming.id,
        "update",
        Some(auth.user_id),
        None,
        Some(json!({ "openingVoucherId": opening_voucher_id })),
    )
    .await
    .map_err(|_| internal_error())?;

    tx.commit().await.map_err(|_| internal_error())?;
    Ok((
        StatusCode::CREATED,
        Json(
            json!({ "closingVoucherId": closing_voucher_id, "openingVoucherId": opening_voucher_id }),
        ),
    ))
}
