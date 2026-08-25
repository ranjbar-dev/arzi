#![allow(clippy::type_complexity)]

pub mod accounts;
pub mod audit;
pub mod auth;
pub mod auto_post;
pub mod backup;
pub mod db;
pub mod deposit_slips;
pub mod export;
pub mod fiscal_years;
pub mod inventory_documents;
pub mod inventory_posting;
pub mod inventory_reports;
pub mod issued_cheques;
pub mod items;
pub mod ledgers;
pub mod money;
pub mod parties;
pub mod party_balance;
pub mod pdf;
pub mod period_close;
pub mod persian_words;
pub mod petty_cash;
pub mod pistachio;
pub mod received_cheques;
pub mod settlement;
pub mod shareholdings;
pub mod sort;
pub mod stock;
pub mod trial_balance;
pub mod vouchers;

use axum::{extract::State, routing::get, Json, Router};
use serde_json::{json, Value};

#[derive(Clone)]
pub struct AppState {
    pub pool: sqlx::PgPool,
}

async fn health(State(state): State<AppState>) -> Json<Value> {
    let db_ok = sqlx::query_scalar::<_, i32>("SELECT 1")
        .fetch_one(&state.pool)
        .await
        .is_ok();
    Json(json!({ "status": "ok", "db": if db_ok { "ok" } else { "unreachable" } }))
}

/// The step 1.2 manual test's "protected endpoint" (401 with no/expired/
/// revoked session, 200 with a valid one), extended in step 1.6 to carry
/// everything the shell's session-aware layout needs in one call: tenant
/// name, username, superuser flag, and the session's current fiscal year
/// (1.5's resolution logic, shared via `fiscal_years::resolve_current_fiscal_year_id`).
async fn me(
    State(state): State<AppState>,
    auth: auth::AuthUser,
) -> Result<Json<Value>, axum::http::StatusCode> {
    let mut tx = db::begin(&state.pool, auth.tenant_id)
        .await
        .map_err(|_| axum::http::StatusCode::INTERNAL_SERVER_ERROR)?;

    let (username, tenant_name): (String, String) = sqlx::query_as(
        "SELECT u.username, t.name FROM users u JOIN tenants t ON t.id = u.tenant_id \
         WHERE u.id = $1",
    )
    .bind(auth.user_id)
    .fetch_one(&mut *tx)
    .await
    .map_err(|_| axum::http::StatusCode::INTERNAL_SERVER_ERROR)?;

    let current_fiscal_year =
        fiscal_years::resolve_current_fiscal_year_id(&mut tx, auth.tenant_id, auth.user_id)
            .await
            .map_err(|_| axum::http::StatusCode::INTERNAL_SERVER_ERROR)?;
    let current_fiscal_year_label: Option<i32> = if let Some(id) = current_fiscal_year {
        sqlx::query_scalar("SELECT year FROM fiscal_years WHERE id = $1")
            .bind(id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(|_| axum::http::StatusCode::INTERNAL_SERVER_ERROR)?
    } else {
        None
    };
    tx.rollback().await.ok();

    Ok(Json(json!({
        "userId": auth.user_id,
        "tenantId": auth.tenant_id,
        "username": username,
        "tenantName": tenant_name,
        "isSuperuser": auth.is_superuser,
        "isPlatformAdmin": auth.is_platform_admin,
        "permissions": auth.permissions,
        "currentFiscalYearId": current_fiscal_year,
        "currentFiscalYear": current_fiscal_year_label,
    })))
}

pub fn app(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/api/v1/me", get(me))
        .nest("/api/v1/auth", auth::router())
        .nest("/api/v1/admin", auth::admin::router())
        .nest("/api/v1/fiscal-years", fiscal_years::router())
        .nest("/api/v1/platform", backup::router())
        .nest("/api/v1/accounts", accounts::router())
        .nest("/api/v1/parties", parties::router())
        .nest("/api/v1/shareholdings", shareholdings::router())
        .nest("/api/v1/vouchers", vouchers::router())
        .nest("/api/v1/received-cheques", received_cheques::router())
        .nest("/api/v1/deposit-slips", deposit_slips::router())
        .nest("/api/v1/petty-cash-claims", petty_cash::router())
        .nest("/api/v1/cheque-payment-batches", issued_cheques::router())
        .nest("/api/v1/warehouses", items::warehouses_router())
        .nest("/api/v1/units-of-measure", items::units_of_measure_router())
        .nest("/api/v1/pistachio-grades", items::pistachio_grades_router())
        .nest("/api/v1/items", items::items_router())
        .nest("/api/v1/inventory-documents", inventory_documents::router())
        .nest("/api/v1/pistachio-deduction", pistachio::router())
        .nest(
            "/api/v1/reports",
            trial_balance::router()
                .merge(ledgers::router())
                .merge(inventory_reports::router())
                .merge(export::router()),
        )
        .with_state(state)
}
