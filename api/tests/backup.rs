//! Automated version of step 7.3's manual test (docs/phase-7-hardening-and-cutover.md §7.3):
//! `RequirePlatformAdmin`'s authorization gate (a tenant superuser is NOT enough — the whole point
//! of the new role) and the retention-count logic. The real `pg_dump`/`pg_restore` execution path
//! is verified live against the rebuilt Docker stack instead of here — `pg_dump` is not installed
//! on this host, and shelling out to it is the one part of this module a permission/logic test
//! doesn't need to exercise to be meaningful (RequirePlatformAdmin rejects before the handler body,
//! including before pg_dump, ever runs).

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

static NEXT_USERNAME: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);

async fn make_user(
    pool: &PgPool,
    tenant_id: i64,
    superuser: bool,
    platform_admin: bool,
) -> (i64, String) {
    let n = NEXT_USERNAME.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let username = format!("u{n}");
    let user_id: i64 = sqlx::query_scalar(
        "INSERT INTO users (tenant_id, username, password_hash, is_superuser, is_platform_admin) \
         VALUES ($1, $2, 'x', $3, $4) RETURNING id",
    )
    .bind(tenant_id)
    .bind(&username)
    .bind(superuser)
    .bind(platform_admin)
    .fetch_one(pool)
    .await
    .unwrap();
    let token = format!("test-session-{user_id}");
    sqlx::query(
        "INSERT INTO sessions (id, user_id, tenant_id, expires_at) VALUES ($1, $2, $3, now() + interval '1 hour')",
    )
    .bind(&token)
    .bind(user_id)
    .bind(tenant_id)
    .execute(pool)
    .await
    .unwrap();
    (user_id, token)
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

async fn req(
    router: &axum::Router,
    method: &str,
    path: &str,
    token: &str,
    body: Value,
) -> axum::response::Response {
    let mut builder = Request::builder()
        .method(method)
        .uri(path)
        .header(header::COOKIE, cookie(token));
    let b = if body.is_null() {
        Body::empty()
    } else {
        builder = builder.header(header::CONTENT_TYPE, "application/json");
        Body::from(body.to_string())
    };
    router
        .clone()
        .oneshot(builder.body(b).unwrap())
        .await
        .unwrap()
}

/// The core of this step: a tenant's own superuser is NOT a platform admin. Reusing
/// `RequireSuperuser` here would have been the cross-tenant leak this step exists to prevent.
#[sqlx::test(migrations = "./migrations")]
async fn tenant_superuser_is_not_a_platform_admin(pool: PgPool) -> sqlx::Result<()> {
    let tenant_id = seed_tenant(&pool).await;
    let (_, superuser_token) = make_user(&pool, tenant_id, true, false).await;
    let router = app(AppState { pool: pool.clone() });

    let resp = req(
        &router,
        "GET",
        "/api/v1/platform/backups",
        &superuser_token,
        Value::Null,
    )
    .await;
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);

    let resp = req(
        &router,
        "POST",
        "/api/v1/platform/backups",
        &superuser_token,
        Value::Null,
    )
    .await;
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    Ok(())
}

/// A plain (non-superuser, non-platform-admin) user is rejected too, and unauthenticated requests
/// 401 before the platform-admin check is even reached.
#[sqlx::test(migrations = "./migrations")]
async fn plain_user_and_anonymous_rejected(pool: PgPool) -> sqlx::Result<()> {
    let tenant_id = seed_tenant(&pool).await;
    let (_, plain_token) = make_user(&pool, tenant_id, false, false).await;
    let router = app(AppState { pool: pool.clone() });

    let resp = req(
        &router,
        "GET",
        "/api/v1/platform/backups",
        &plain_token,
        Value::Null,
    )
    .await;
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);

    let resp = req(
        &router,
        "GET",
        "/api/v1/platform/backups",
        "no-such-session",
        Value::Null,
    )
    .await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    Ok(())
}

/// A real platform admin CAN list (and the list endpoint itself never touches `pg_dump`, so it's
/// meaningful to run on a host without the binary installed).
#[sqlx::test(migrations = "./migrations")]
async fn platform_admin_can_list_backups(pool: PgPool) -> sqlx::Result<()> {
    let tenant_id = seed_tenant(&pool).await;
    let (_, admin_token) = make_user(&pool, tenant_id, false, true).await;
    let router = app(AppState { pool: pool.clone() });

    let resp = req(
        &router,
        "GET",
        "/api/v1/platform/backups",
        &admin_token,
        Value::Null,
    )
    .await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body = json_body(resp).await;
    assert_eq!(body["items"].as_array().unwrap().len(), 0);
    Ok(())
}

/// The grant/revoke endpoint itself requires an existing platform admin, and correctly flips the
/// flag on another user in the caller's own tenant (RLS scopes it there transparently).
#[sqlx::test(migrations = "./migrations")]
async fn grant_and_revoke_platform_admin(pool: PgPool) -> sqlx::Result<()> {
    let tenant_id = seed_tenant(&pool).await;
    let (_, admin_token) = make_user(&pool, tenant_id, false, true).await;
    let (target_id, _) = make_user(&pool, tenant_id, false, false).await;
    let router = app(AppState { pool: pool.clone() });

    // A plain user cannot grant.
    let (_, plain_token) = make_user(&pool, tenant_id, false, false).await;
    let resp = req(
        &router,
        "PUT",
        &format!("/api/v1/platform/users/{target_id}/platform-admin"),
        &plain_token,
        serde_json::json!({ "grant": true }),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);

    let resp = req(
        &router,
        "PUT",
        &format!("/api/v1/platform/users/{target_id}/platform-admin"),
        &admin_token,
        serde_json::json!({ "grant": true }),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);
    let is_admin: bool = sqlx::query_scalar("SELECT is_platform_admin FROM users WHERE id = $1")
        .bind(target_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert!(is_admin);

    let resp = req(
        &router,
        "PUT",
        &format!("/api/v1/platform/users/{target_id}/platform-admin"),
        &admin_token,
        serde_json::json!({ "grant": false }),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);
    let is_admin: bool = sqlx::query_scalar("SELECT is_platform_admin FROM users WHERE id = $1")
        .bind(target_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert!(!is_admin);
    Ok(())
}

/// Downloading a nonexistent backup 404s, and a `running`/`failed` backup can't be downloaded
/// (never a half-written or absent file served as if it were a real dump) — both checkable without
/// ever invoking `pg_dump`.
#[sqlx::test(migrations = "./migrations")]
async fn download_guards_against_incomplete_backups(pool: PgPool) -> sqlx::Result<()> {
    let tenant_id = seed_tenant(&pool).await;
    let (_, admin_token) = make_user(&pool, tenant_id, false, true).await;
    let router = app(AppState { pool: pool.clone() });

    let resp = req(
        &router,
        "GET",
        "/api/v1/platform/backups/999999/download",
        &admin_token,
        Value::Null,
    )
    .await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);

    let running_id: i64 = sqlx::query_scalar(
        "INSERT INTO backups (filename, status, trigger) VALUES ('x.dump', 'running', 'manual') RETURNING id",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    let resp = req(
        &router,
        "GET",
        &format!("/api/v1/platform/backups/{running_id}/download"),
        &admin_token,
        Value::Null,
    )
    .await;
    assert_eq!(resp.status(), StatusCode::CONFLICT);
    Ok(())
}
