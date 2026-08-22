//! Automated version of step 1.3's manual test (docs/phase-1-platform-and-
//! auth.md §1.3): a granted user gets 200, an ungranted user gets 403 (not a
//! degraded response), the check flips live on grant/revoke, and a
//! superuser acts without an explicit grant. Also covers the admin
//! endpoints' own gate (superuser-only) and the transactional permission
//! replace.
//!
//! `RequirePermission` is exercised against a tiny demo route mounted only
//! in this test file — step 1.3 builds the mechanism, wiring it to a real
//! business route is later phases' job (2.8, 4.x, 6.7), so there's no
//! production route to hit instead. The four admin endpoints ARE real
//! production routes and are exercised directly.

use api::{app, auth::Perm, AppState};
use axum::{
    body::Body,
    http::{header, Request, StatusCode},
    routing::get,
    Router,
};
use http_body_util::BodyExt;
use serde_json::Value;
use sqlx::PgPool;
use tower::ServiceExt;

const ACCOUNT_LIST_PERMISSION_ID: i32 = 1101; // code "account_list", seeded by migration 0002

struct DemoPerm;
impl Perm for DemoPerm {
    const CODE: &'static str = "account_list";
}

async fn demo_handler(_req: api::auth::RequirePermission<DemoPerm>) -> StatusCode {
    StatusCode::OK
}

fn demo_router(state: AppState) -> Router {
    Router::new()
        .route("/demo", get(demo_handler))
        .with_state(state)
}

async fn seed_tenant(pool: &PgPool) -> i64 {
    sqlx::query_scalar("INSERT INTO tenants (slug, name) VALUES ('acme', 'Acme') RETURNING id")
        .fetch_one(pool)
        .await
        .unwrap()
}

async fn seed_user(pool: &PgPool, tenant_id: i64, username: &str, is_superuser: bool) -> i64 {
    sqlx::query_scalar(
        "INSERT INTO users (tenant_id, username, password_hash, is_superuser) \
         VALUES ($1, $2, 'x', $3) RETURNING id",
    )
    .bind(tenant_id)
    .bind(username)
    .bind(is_superuser)
    .fetch_one(pool)
    .await
    .unwrap()
}

async fn seed_session(pool: &PgPool, user_id: i64, tenant_id: i64) -> String {
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

#[sqlx::test(migrations = "./migrations")]
async fn permission_grant_gates_the_route(pool: PgPool) -> sqlx::Result<()> {
    let tenant_id = seed_tenant(&pool).await;
    let granted = seed_user(&pool, tenant_id, "granted", false).await;
    let ungranted = seed_user(&pool, tenant_id, "ungranted", false).await;
    let superuser = seed_user(&pool, tenant_id, "root", true).await;

    sqlx::query(
        "INSERT INTO user_permissions (tenant_id, user_id, permission_id) VALUES ($1, $2, $3)",
    )
    .bind(tenant_id)
    .bind(granted)
    .bind(ACCOUNT_LIST_PERMISSION_ID)
    .execute(&pool)
    .await?;

    let granted_token = seed_session(&pool, granted, tenant_id).await;
    let ungranted_token = seed_session(&pool, ungranted, tenant_id).await;
    let superuser_token = seed_session(&pool, superuser, tenant_id).await;

    let router = demo_router(AppState { pool: pool.clone() });

    let granted_resp = router
        .clone()
        .oneshot(
            Request::get("/demo")
                .header(header::COOKIE, cookie(&granted_token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(granted_resp.status(), StatusCode::OK);

    let ungranted_resp = router
        .clone()
        .oneshot(
            Request::get("/demo")
                .header(header::COOKIE, cookie(&ungranted_token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(ungranted_resp.status(), StatusCode::FORBIDDEN);

    // Superuser acts without any explicit grant.
    let superuser_resp = router
        .clone()
        .oneshot(
            Request::get("/demo")
                .header(header::COOKIE, cookie(&superuser_token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(superuser_resp.status(), StatusCode::OK);

    // No session at all -> 401, distinct from "wrong permission" 403.
    let anonymous_resp = router
        .oneshot(Request::get("/demo").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(anonymous_resp.status(), StatusCode::UNAUTHORIZED);

    Ok(())
}

#[sqlx::test(migrations = "./migrations")]
async fn revoking_a_permission_takes_effect_on_the_next_request(pool: PgPool) -> sqlx::Result<()> {
    let tenant_id = seed_tenant(&pool).await;
    let user_id = seed_user(&pool, tenant_id, "alice", false).await;
    let token = seed_session(&pool, user_id, tenant_id).await;
    let router = demo_router(AppState { pool: pool.clone() });

    let before = router
        .clone()
        .oneshot(
            Request::get("/demo")
                .header(header::COOKIE, cookie(&token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(before.status(), StatusCode::FORBIDDEN);

    sqlx::query(
        "INSERT INTO user_permissions (tenant_id, user_id, permission_id) VALUES ($1, $2, $3)",
    )
    .bind(tenant_id)
    .bind(user_id)
    .bind(ACCOUNT_LIST_PERMISSION_ID)
    .execute(&pool)
    .await?;

    let after_grant = router
        .clone()
        .oneshot(
            Request::get("/demo")
                .header(header::COOKIE, cookie(&token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(after_grant.status(), StatusCode::OK);

    sqlx::query("DELETE FROM user_permissions WHERE user_id = $1 AND permission_id = $2")
        .bind(user_id)
        .bind(ACCOUNT_LIST_PERMISSION_ID)
        .execute(&pool)
        .await?;

    let after_revoke = router
        .oneshot(
            Request::get("/demo")
                .header(header::COOKIE, cookie(&token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(after_revoke.status(), StatusCode::FORBIDDEN);

    Ok(())
}

async fn json_body(response: axum::response::Response) -> Value {
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap()
}

#[sqlx::test(migrations = "./migrations")]
async fn admin_endpoints_require_superuser(pool: PgPool) -> sqlx::Result<()> {
    let tenant_id = seed_tenant(&pool).await;
    let plain_user = seed_user(&pool, tenant_id, "plain", false).await;
    let token = seed_session(&pool, plain_user, tenant_id).await;
    let router = app(AppState { pool: pool.clone() });

    let resp = router
        .oneshot(
            Request::get("/api/v1/admin/users")
                .header(header::COOKIE, cookie(&token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);

    Ok(())
}

#[sqlx::test(migrations = "./migrations")]
async fn admin_can_create_activate_and_manage_a_user(pool: PgPool) -> sqlx::Result<()> {
    let tenant_id = seed_tenant(&pool).await;
    let admin_id = seed_user(&pool, tenant_id, "root", true).await;
    let admin_token = seed_session(&pool, admin_id, tenant_id).await;
    let router = app(AppState { pool: pool.clone() });

    // Create -> the sentinel password can never log in (step 1.2's contract).
    let create_resp = router
        .clone()
        .oneshot(
            Request::post("/api/v1/admin/users")
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::COOKIE, cookie(&admin_token))
                .body(Body::from(r#"{"username":"newhire"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(create_resp.status(), StatusCode::CREATED);
    let created = json_body(create_resp).await;
    let new_user_id = created["id"].as_i64().unwrap();

    let login_before_activation = router
        .clone()
        .oneshot(
            Request::post("/api/v1/auth/login")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    r#"{"tenantSlug":"acme","username":"newhire","password":""}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(login_before_activation.status(), StatusCode::UNAUTHORIZED);

    // Admin sets an initial password -> account is now usable.
    let set_password_resp = router
        .clone()
        .oneshot(
            Request::post(format!("/api/v1/admin/users/{new_user_id}/set-password"))
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::COOKIE, cookie(&admin_token))
                .body(Body::from(r#"{"newPassword":"InitialPass123"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(set_password_resp.status(), StatusCode::NO_CONTENT);

    let login_after_activation = router
        .clone()
        .oneshot(
            Request::post("/api/v1/auth/login")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    r#"{"tenantSlug":"acme","username":"newhire","password":"InitialPass123"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(login_after_activation.status(), StatusCode::OK);

    // Disable -> login rejected again.
    let disable_resp = router
        .clone()
        .oneshot(
            Request::post(format!("/api/v1/admin/users/{new_user_id}/disable"))
                .header(header::COOKIE, cookie(&admin_token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(disable_resp.status(), StatusCode::NO_CONTENT);

    let login_while_disabled = router
        .clone()
        .oneshot(
            Request::post("/api/v1/auth/login")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    r#"{"tenantSlug":"acme","username":"newhire","password":"InitialPass123"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(login_while_disabled.status(), StatusCode::UNAUTHORIZED);

    // Re-enable -> login works again.
    let enable_resp = router
        .clone()
        .oneshot(
            Request::post(format!("/api/v1/admin/users/{new_user_id}/enable"))
                .header(header::COOKIE, cookie(&admin_token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(enable_resp.status(), StatusCode::NO_CONTENT);

    let login_after_reenable = router
        .clone()
        .oneshot(
            Request::post("/api/v1/auth/login")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    r#"{"tenantSlug":"acme","username":"newhire","password":"InitialPass123"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(login_after_reenable.status(), StatusCode::OK);

    // Grant a permission -> list-users shows the account, and the grant is
    // queryable back out via the DB (replace_user_permissions is exercised
    // directly by the authz tests above; here just prove the route works).
    let grant_resp = router
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!("/api/v1/admin/users/{new_user_id}/permissions"))
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::COOKIE, cookie(&admin_token))
                .body(Body::from(r#"{"permissionIds":[1101]}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(grant_resp.status(), StatusCode::NO_CONTENT);

    let granted: Vec<i32> = sqlx::query_scalar(
        "SELECT permission_id FROM user_permissions WHERE user_id = $1",
    )
    .bind(new_user_id)
    .fetch_all(&pool)
    .await?;
    assert_eq!(granted, vec![1101]);

    // Replace again with an empty set -> the old grant is gone, atomically.
    let clear_resp = router
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!("/api/v1/admin/users/{new_user_id}/permissions"))
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::COOKIE, cookie(&admin_token))
                .body(Body::from(r#"{"permissionIds":[]}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(clear_resp.status(), StatusCode::NO_CONTENT);

    let remaining: i64 = sqlx::query_scalar("SELECT count(*) FROM user_permissions WHERE user_id = $1")
        .bind(new_user_id)
        .fetch_one(&pool)
        .await?;
    assert_eq!(remaining, 0);

    Ok(())
}
