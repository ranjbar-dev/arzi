//! Step 1.6: `/api/v1/me` grew tenant/username/fiscal-year fields so the
//! frontend shell can render them from one call.

use api::{app, AppState};
use axum::{
    body::Body,
    http::{header, Request, StatusCode},
};
use serde_json::Value;
use sqlx::PgPool;
use tower::ServiceExt;

async fn json_body(response: axum::response::Response) -> Value {
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

#[sqlx::test(migrations = "./migrations")]
async fn me_reports_tenant_user_and_current_fiscal_year(pool: PgPool) -> sqlx::Result<()> {
    let tenant_id: i64 = sqlx::query_scalar(
        "INSERT INTO tenants (slug, name) VALUES ('acme', 'Acme Co') RETURNING id",
    )
    .fetch_one(&pool)
    .await?;
    let user_id: i64 = sqlx::query_scalar(
        "INSERT INTO users (tenant_id, username, password_hash, is_superuser) \
         VALUES ($1, 'root', 'x', true) RETURNING id",
    )
    .bind(tenant_id)
    .fetch_one(&pool)
    .await?;
    let fiscal_year_id: i64 = sqlx::query_scalar(
        "INSERT INTO fiscal_years (tenant_id, year, start_date, end_date) \
         VALUES ($1, 1403, '2024-03-20', '2025-03-20') RETURNING id",
    )
    .bind(tenant_id)
    .fetch_one(&pool)
    .await?;
    let token = "test-session-me";
    sqlx::query(
        "INSERT INTO sessions (id, user_id, tenant_id, expires_at) \
         VALUES ($1, $2, $3, now() + interval '1 hour')",
    )
    .bind(token)
    .bind(user_id)
    .bind(tenant_id)
    .execute(&pool)
    .await?;

    let router = app(AppState { pool });
    let resp = router
        .oneshot(
            Request::get("/api/v1/me")
                .header(header::COOKIE, format!("arzi_session={token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = json_body(resp).await;
    assert_eq!(body["username"], "root");
    assert_eq!(body["tenantName"], "Acme Co");
    assert_eq!(body["isSuperuser"], true);
    assert_eq!(body["currentFiscalYearId"], fiscal_year_id);
    assert_eq!(body["currentFiscalYear"], 1403);

    Ok(())
}
