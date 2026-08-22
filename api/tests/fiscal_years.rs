//! Automated version of step 1.5's manual test (docs/phase-1-platform-and-
//! auth.md §1.5): create a fiscal year, reject an overlap, create a second
//! non-overlapping year, switch the session's current year, close a year.

use api::{app, AppState};
use axum::{
    body::Body,
    http::{header, Request, StatusCode},
};
use serde_json::Value;
use sqlx::PgPool;
use tower::ServiceExt;

async fn seed_tenant(pool: &PgPool) -> i64 {
    sqlx::query_scalar("INSERT INTO tenants (slug, name) VALUES ('acme', 'Acme') RETURNING id")
        .fetch_one(pool)
        .await
        .unwrap()
}

async fn seed_session(pool: &PgPool, tenant_id: i64, username: &str, is_superuser: bool) -> String {
    let user_id: i64 = sqlx::query_scalar(
        "INSERT INTO users (tenant_id, username, password_hash, is_superuser) \
         VALUES ($1, $2, 'x', $3) RETURNING id",
    )
    .bind(tenant_id)
    .bind(username)
    .bind(is_superuser)
    .fetch_one(pool)
    .await
    .unwrap();
    let token = format!("test-session-{user_id}");
    sqlx::query(
        "INSERT INTO sessions (id, user_id, tenant_id, expires_at) \
         VALUES ($1, $2, $3, now() + interval '1 hour')",
    )
    .bind(&token)
    .bind(user_id)
    .bind(tenant_id)
    .execute(pool)
    .await
    .unwrap();
    token
}

fn cookie(token: &str) -> String {
    format!("arzi_session={token}")
}

async fn json_body(response: axum::response::Response) -> Value {
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

#[sqlx::test(migrations = "./migrations")]
async fn create_reject_overlap_create_close(pool: PgPool) -> sqlx::Result<()> {
    let tenant_id = seed_tenant(&pool).await;
    let admin_token = seed_session(&pool, tenant_id, "root", true).await;
    let router = app(AppState { pool: pool.clone() });

    // 1. Create fiscal year 1403.
    let create_1403 = router
        .clone()
        .oneshot(
            Request::post("/api/v1/fiscal-years")
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::COOKIE, cookie(&admin_token))
                .body(Body::from(
                    r#"{"year":1403,"startDate":"2024-03-20","endDate":"2025-03-20"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(create_1403.status(), StatusCode::CREATED);
    let year_1403_id = json_body(create_1403).await["id"].as_i64().unwrap();

    // 2. Overlapping range for the same tenant -> rejected.
    let overlap = router
        .clone()
        .oneshot(
            Request::post("/api/v1/fiscal-years")
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::COOKIE, cookie(&admin_token))
                .body(Body::from(
                    r#"{"year":1499,"startDate":"2024-06-01","endDate":"2025-06-01"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(overlap.status(), StatusCode::CONFLICT);

    // 3. Non-overlapping fiscal year 1404 -> succeeds.
    let create_1404 = router
        .clone()
        .oneshot(
            Request::post("/api/v1/fiscal-years")
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::COOKIE, cookie(&admin_token))
                .body(Body::from(
                    r#"{"year":1404,"startDate":"2025-03-21","endDate":"2026-03-20"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(create_1404.status(), StatusCode::CREATED);
    let year_1404_id = json_body(create_1404).await["id"].as_i64().unwrap();

    // 4. Switch the session's current fiscal year to 1404, confirm it reports back.
    let switch = router
        .clone()
        .oneshot(
            Request::put("/api/v1/fiscal-years/current")
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::COOKIE, cookie(&admin_token))
                .body(Body::from(format!(r#"{{"fiscalYearId":{year_1404_id}}}"#)))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(switch.status(), StatusCode::NO_CONTENT);

    let current = router
        .clone()
        .oneshot(
            Request::get("/api/v1/fiscal-years/current")
                .header(header::COOKIE, cookie(&admin_token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(
        json_body(current).await["fiscalYearId"].as_i64(),
        Some(year_1404_id)
    );

    // 5. Close fiscal year 1403 -> is_active flips to false.
    let close = router
        .clone()
        .oneshot(
            Request::post(format!("/api/v1/fiscal-years/{year_1403_id}/close"))
                .header(header::COOKIE, cookie(&admin_token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(close.status(), StatusCode::NO_CONTENT);

    let list = router
        .oneshot(
            Request::get("/api/v1/fiscal-years")
                .header(header::COOKIE, cookie(&admin_token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let years = json_body(list).await;
    let closed = years
        .as_array()
        .unwrap()
        .iter()
        .find(|y| y["id"].as_i64() == Some(year_1403_id))
        .unwrap();
    assert_eq!(closed["isActive"], false);

    // Closing again is rejected, not a silent no-op flip.
    let close_again = router_reclone(&pool)
        .oneshot(
            Request::post(format!("/api/v1/fiscal-years/{year_1403_id}/close"))
                .header(header::COOKIE, cookie(&admin_token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(close_again.status(), StatusCode::CONFLICT);

    Ok(())
}

fn router_reclone(pool: &PgPool) -> axum::Router {
    app(AppState { pool: pool.clone() })
}

#[sqlx::test(migrations = "./migrations")]
async fn create_and_close_require_superuser(pool: PgPool) -> sqlx::Result<()> {
    let tenant_id = seed_tenant(&pool).await;
    let plain_token = seed_session(&pool, tenant_id, "plain", false).await;
    let router = app(AppState { pool: pool.clone() });

    let resp = router
        .oneshot(
            Request::post("/api/v1/fiscal-years")
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::COOKIE, cookie(&plain_token))
                .body(Body::from(
                    r#"{"year":1403,"startDate":"2024-03-20","endDate":"2025-03-20"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);

    Ok(())
}
