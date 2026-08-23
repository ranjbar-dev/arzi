//! Automated version of step 1.2's manual test (docs/phase-1-platform-and-
//! auth.md §1.2): login issues a real session cookie, wrong credentials get
//! a generic failure regardless of which check failed, the protected `/me`
//! endpoint honours the session, logout revokes it, and a sentinel
//! ("no usable password") account can never log in.
//!
//! Uses the plain #[sqlx::test] owner pool directly (no SET ROLE, unlike
//! `rls_tenant_isolation.rs`) — step 1.1 already proves RLS blocks
//! cross-tenant access at the Postgres level; this test is about the auth
//! flow itself, and the login query filters by `tenant_id` explicitly
//! regardless of which role runs it. Run with:
//!   DATABASE_URL=postgres://arzi:change_me@localhost:5432/arzi cargo test -p api

use api::{app, auth, AppState};
use axum::{
    body::Body,
    http::{header, Request, StatusCode},
};
use http_body_util::BodyExt;
use sqlx::PgPool;
use tower::ServiceExt;

async fn seed_tenant_and_user(pool: &PgPool, password_hash: &str) -> (String, String) {
    let tenant_slug = "acme";
    sqlx::query("INSERT INTO tenants (slug, name) VALUES ($1, 'Acme')")
        .bind(tenant_slug)
        .execute(pool)
        .await
        .unwrap();
    let tenant_id: i64 = sqlx::query_scalar("SELECT id FROM tenants WHERE slug = $1")
        .bind(tenant_slug)
        .fetch_one(pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO users (tenant_id, username, password_hash) VALUES ($1, 'alice', $2)")
        .bind(tenant_id)
        .bind(password_hash)
        .execute(pool)
        .await
        .unwrap();
    (tenant_slug.to_string(), "alice".to_string())
}

fn login_body(tenant_slug: &str, username: &str, password: &str) -> String {
    serde_json::json!({ "tenantSlug": tenant_slug, "username": username, "password": password })
        .to_string()
}

fn extract_cookie(response: &axum::response::Response) -> String {
    let raw = response
        .headers()
        .get(header::SET_COOKIE)
        .expect("Set-Cookie header missing")
        .to_str()
        .unwrap();
    raw.split(';').next().unwrap().to_string() // "arzi_session=<token>"
}

#[sqlx::test(migrations = "./migrations")]
async fn login_session_lifecycle(pool: PgPool) -> sqlx::Result<()> {
    let hash = auth::hash_password("correct horse battery staple");
    let (tenant_slug, username) = seed_tenant_and_user(&pool, &hash).await;
    let router = app(AppState { pool: pool.clone() });

    // Wrong password -> generic 401.
    let bad = router
        .clone()
        .oneshot(
            Request::post("/api/v1/auth/login")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(login_body(&tenant_slug, &username, "wrong")))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(bad.status(), StatusCode::UNAUTHORIZED);
    let bad_body = bad.into_body().collect().await.unwrap().to_bytes();

    // Unknown username in the same tenant -> identical generic failure body.
    let unknown = router
        .clone()
        .oneshot(
            Request::post("/api/v1/auth/login")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(login_body(&tenant_slug, "nobody", "wrong")))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(unknown.status(), StatusCode::UNAUTHORIZED);
    let unknown_body = unknown.into_body().collect().await.unwrap().to_bytes();
    assert_eq!(
        bad_body, unknown_body,
        "login failure must not distinguish the reason"
    );

    // Correct credentials -> 200 + session cookie.
    let ok = router
        .clone()
        .oneshot(
            Request::post("/api/v1/auth/login")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(login_body(
                    &tenant_slug,
                    &username,
                    "correct horse battery staple",
                )))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(ok.status(), StatusCode::OK);
    let cookie = extract_cookie(&ok);

    // /me without a cookie -> 401.
    let unauthenticated = router
        .clone()
        .oneshot(Request::get("/api/v1/me").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(unauthenticated.status(), StatusCode::UNAUTHORIZED);

    // /me with the session cookie -> 200.
    let authenticated = router
        .clone()
        .oneshot(
            Request::get("/api/v1/me")
                .header(header::COOKIE, cookie.clone())
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(authenticated.status(), StatusCode::OK);

    // Logout, then the same cookie is rejected.
    let logout = router
        .clone()
        .oneshot(
            Request::post("/api/v1/auth/logout")
                .header(header::COOKIE, cookie.clone())
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(logout.status(), StatusCode::NO_CONTENT);

    let after_logout = router
        .clone()
        .oneshot(
            Request::get("/api/v1/me")
                .header(header::COOKIE, cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(after_logout.status(), StatusCode::UNAUTHORIZED);

    Ok(())
}

#[sqlx::test(migrations = "./migrations")]
async fn sentinel_password_can_never_log_in(pool: PgPool) -> sqlx::Result<()> {
    let (tenant_slug, username) = seed_tenant_and_user(&pool, auth::NO_PASSWORD_SENTINEL).await;
    let router = app(AppState { pool });

    let attempt = router
        .oneshot(
            Request::post("/api/v1/auth/login")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(login_body(&tenant_slug, &username, "")))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(attempt.status(), StatusCode::UNAUTHORIZED);

    Ok(())
}
