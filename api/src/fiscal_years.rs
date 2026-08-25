//! Step 1.5 (docs/phase-1-platform-and-auth.md §1.5): explicit, auditable
//! fiscal-year create/close, plus the "current fiscal year" a user is acting
//! in right now — every later domain read/write is implicitly scoped to it,
//! the same way legacy `CO_ID` scoped everything (`00-overview.md` fact 1).
//!
//! A5 (`specs/11-open-decisions.md`) — closing is a deliberate admin action,
//! nothing ever flips `is_active` implicitly. A6 — chart of accounts stays
//! global, no per-year copy on create. Both already decided; not
//! re-litigated here.
//!
//! Create/close are gated `RequireSuperuser`, matching the legacy's own shape
//! for this class of action (`08-04-authorization.md` §4.4's "Supervisor-
//! only, no Pass_Config id" bucket includes `B_Enteghal1`, year-end closing —
//! there is no catalogue permission for fiscal-year lifecycle at all).
//! Switching *which* fiscal year a session is currently acting in is not a
//! privileged action — it's per-user UI state, stored in 1.1's generic
//! `user_preferences` table rather than a bespoke column.

use crate::{
    audit,
    auth::{authz::RequireSuperuser, AuthUser},
    db, period_close, AppState,
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

const CURRENT_FISCAL_YEAR_PREF: (&str, &str) = ("ui", "current_fiscal_year_id");

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(list_fiscal_years).post(create_fiscal_year))
        .route(
            "/current",
            get(get_current_fiscal_year).put(set_current_fiscal_year),
        )
        .route("/{id}/close", post(close_fiscal_year))
        // step 2.7 (NewFinalu / EnteghalU equivalents) — kept here rather than a separate
        // top-level mount since both take a fiscal-year id as their subject.
        .route("/{id}/close-books", post(period_close::close_books))
        .route("/{id}/carry-forward", post(period_close::carry_forward))
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct FiscalYearRow {
    id: i64,
    year: i32,
    start_date: NaiveDate,
    end_date: NaiveDate,
    is_active: bool,
}

fn internal_error() -> (StatusCode, Json<Value>) {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(json!({ "error": "internal_error" })),
    )
}

#[derive(Deserialize)]
struct ListQuery {
    year: Option<String>,
    sort: Option<String>,
    order: Option<String>,
}

const FISCAL_YEAR_SORT_COLUMNS: &[(&str, &str)] = &[
    ("year", "year"),
    ("startDate", "start_date"),
    ("endDate", "end_date"),
    ("status", "is_active"),
];

async fn list_fiscal_years(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(q): Query<ListQuery>,
) -> Result<Json<Vec<FiscalYearRow>>, (StatusCode, Json<Value>)> {
    let mut tx = db::begin(&state.pool, auth.tenant_id)
        .await
        .map_err(|_| internal_error())?;

    let rows: Vec<(i64, i32, NaiveDate, NaiveDate, bool)> = sqlx::query_as(&format!(
        "SELECT id, year, start_date, end_date, is_active \
         FROM fiscal_years WHERE tenant_id = $1 \
         AND ($2::text IS NULL OR year::text ILIKE '%' || $2 || '%') \
         ORDER BY {}",
        crate::sort::order_by(
            q.sort.as_deref(),
            q.order.as_deref(),
            FISCAL_YEAR_SORT_COLUMNS,
            "start_date"
        ),
    ))
    .bind(auth.tenant_id)
    .bind(&q.year)
    .fetch_all(&mut *tx)
    .await
    .map_err(|_| internal_error())?;
    tx.rollback().await.ok();

    Ok(Json(
        rows.into_iter()
            .map(
                |(id, year, start_date, end_date, is_active)| FiscalYearRow {
                    id,
                    year,
                    start_date,
                    end_date,
                    is_active,
                },
            )
            .collect(),
    ))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateFiscalYearRequest {
    year: i32,
    start_date: NaiveDate,
    end_date: NaiveDate,
}

async fn create_fiscal_year(
    State(state): State<AppState>,
    admin: RequireSuperuser,
    Json(req): Json<CreateFiscalYearRequest>,
) -> Result<(StatusCode, Json<Value>), (StatusCode, Json<Value>)> {
    if req.end_date <= req.start_date {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "invalid_date_range" })),
        ));
    }

    let mut tx = db::begin(&state.pool, admin.0.tenant_id)
        .await
        .map_err(|_| internal_error())?;

    let id: i64 = sqlx::query_scalar(
        "INSERT INTO fiscal_years (tenant_id, year, start_date, end_date, created_by) \
         VALUES ($1, $2, $3, $4, $5) RETURNING id",
    )
    .bind(admin.0.tenant_id)
    .bind(req.year)
    .bind(req.start_date)
    .bind(req.end_date)
    .bind(admin.0.user_id)
    .fetch_one(&mut *tx)
    .await
    .map_err(fiscal_year_conflict_or_internal)?;

    audit::record_mutation(
        &mut tx,
        admin.0.tenant_id,
        "fiscal_years",
        id,
        "insert",
        Some(admin.0.user_id),
        None,
        Some(json!({ "year": req.year, "startDate": req.start_date, "endDate": req.end_date })),
    )
    .await
    .map_err(|_| internal_error())?;

    tx.commit().await.map_err(|_| internal_error())?;
    Ok((StatusCode::CREATED, Json(json!({ "id": id }))))
}

/// `fiscal_years_year_key` (23505 unique_violation) and
/// `fiscal_years_no_overlap` (23P01 exclusion_violation) are both DB-enforced
/// per-tenant invariants — this repo's DDL is the source of truth, not a
/// duplicated app-side check (`fiscal_years_date_order` is instead checked
/// above in Rust, before the round-trip, since it needs no other row).
fn fiscal_year_conflict_or_internal(err: sqlx::Error) -> (StatusCode, Json<Value>) {
    if let sqlx::Error::Database(db_err) = &err {
        match db_err.code().as_deref() {
            Some("23505") => {
                return (
                    StatusCode::CONFLICT,
                    Json(json!({ "error": "year_already_exists" })),
                )
            }
            Some("23P01") => {
                return (
                    StatusCode::CONFLICT,
                    Json(json!({ "error": "date_range_overlaps" })),
                )
            }
            _ => {}
        }
    }
    internal_error()
}

async fn close_fiscal_year(
    State(state): State<AppState>,
    admin: RequireSuperuser,
    Path(id): Path<i64>,
) -> Result<StatusCode, (StatusCode, Json<Value>)> {
    let mut tx = db::begin(&state.pool, admin.0.tenant_id)
        .await
        .map_err(|_| internal_error())?;

    let was_active: Option<bool> =
        sqlx::query_scalar("SELECT is_active FROM fiscal_years WHERE id = $1")
            .bind(id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(|_| internal_error())?;
    let Some(was_active) = was_active else {
        return Err((
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "fiscal_year_not_found" })),
        ));
    };
    if !was_active {
        return Err((
            StatusCode::CONFLICT,
            Json(json!({ "error": "already_closed" })),
        ));
    }

    // No carry-forward here (that's step 2.7's year-end flow, closing_voucher_id/
    // opening_voucher_id already exist on the row for it to fill in).
    sqlx::query("UPDATE fiscal_years SET is_active = false, closed_at = now(), updated_at = now(), updated_by = $1 WHERE id = $2")
        .bind(admin.0.user_id)
        .bind(id)
        .execute(&mut *tx)
        .await
        .map_err(|_| internal_error())?;

    audit::record_mutation(
        &mut tx,
        admin.0.tenant_id,
        "fiscal_years",
        id,
        "update",
        Some(admin.0.user_id),
        Some(json!({ "isActive": true })),
        Some(json!({ "isActive": false })),
    )
    .await
    .map_err(|_| internal_error())?;

    tx.commit().await.map_err(|_| internal_error())?;
    Ok(StatusCode::NO_CONTENT)
}

/// Shared with `lib::me` (1.6's session-aware shell needs the current fiscal
/// year alongside tenant/user identity in one place) — same resolution order
/// as the route handler below: an explicit `user_preferences` choice first,
/// else the most-recently-started still-open year.
pub(crate) async fn resolve_current_fiscal_year_id(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    tenant_id: i64,
    user_id: i64,
) -> Result<Option<i64>, sqlx::Error> {
    let (scope, key) = CURRENT_FISCAL_YEAR_PREF;
    let stored: Option<Value> = sqlx::query_scalar(
        "SELECT value FROM user_preferences WHERE user_id = $1 AND scope = $2 AND key = $3",
    )
    .bind(user_id)
    .bind(scope)
    .bind(key)
    .fetch_optional(&mut **tx)
    .await?;

    if let Some(value) = stored {
        return Ok(value.get("fiscalYearId").and_then(Value::as_i64));
    }

    sqlx::query_scalar(
        "SELECT id FROM fiscal_years WHERE tenant_id = $1 AND is_active \
         ORDER BY start_date DESC LIMIT 1",
    )
    .bind(tenant_id)
    .fetch_optional(&mut **tx)
    .await
}

async fn get_current_fiscal_year(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let mut tx = db::begin(&state.pool, auth.tenant_id)
        .await
        .map_err(|_| internal_error())?;

    let fiscal_year_id = resolve_current_fiscal_year_id(&mut tx, auth.tenant_id, auth.user_id)
        .await
        .map_err(|_| internal_error())?;
    tx.rollback().await.ok();

    Ok(Json(json!({ "fiscalYearId": fiscal_year_id })))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SetCurrentFiscalYearRequest {
    fiscal_year_id: i64,
}

async fn set_current_fiscal_year(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<SetCurrentFiscalYearRequest>,
) -> Result<StatusCode, (StatusCode, Json<Value>)> {
    let mut tx = db::begin(&state.pool, auth.tenant_id)
        .await
        .map_err(|_| internal_error())?;

    let exists: Option<i64> =
        sqlx::query_scalar("SELECT id FROM fiscal_years WHERE id = $1 AND tenant_id = $2")
            .bind(req.fiscal_year_id)
            .bind(auth.tenant_id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(|_| internal_error())?;
    if exists.is_none() {
        return Err((
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "fiscal_year_not_found" })),
        ));
    }

    let (scope, key) = CURRENT_FISCAL_YEAR_PREF;
    sqlx::query(
        "INSERT INTO user_preferences (tenant_id, user_id, scope, key, value, updated_at) \
         VALUES ($1, $2, $3, $4, $5, now()) \
         ON CONFLICT (user_id, scope, key) DO UPDATE SET value = EXCLUDED.value, updated_at = now()",
    )
    .bind(auth.tenant_id)
    .bind(auth.user_id)
    .bind(scope)
    .bind(key)
    .bind(json!({ "fiscalYearId": req.fiscal_year_id }))
    .execute(&mut *tx)
    .await
    .map_err(|_| internal_error())?;

    tx.commit().await.map_err(|_| internal_error())?;
    Ok(StatusCode::NO_CONTENT)
}
