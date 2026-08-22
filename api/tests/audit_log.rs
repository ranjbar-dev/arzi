//! Automated version of step 1.4's manual test (docs/phase-1-platform-and-
//! auth.md §1.4): create/update a user through the real API, a wrong-
//! password login, and a permission grant each leave a correctly-attributed
//! `audit_log` row.

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

async fn seed_superuser(pool: &PgPool, tenant_id: i64) -> (i64, String) {
    let id: i64 = sqlx::query_scalar(
        "INSERT INTO users (tenant_id, username, password_hash, is_superuser) \
         VALUES ($1, 'root', 'x', true) RETURNING id",
    )
    .bind(tenant_id)
    .fetch_one(pool)
    .await
    .unwrap();
    let token = format!("test-session-{id}");
    sqlx::query(
        "INSERT INTO sessions (id, user_id, tenant_id, expires_at) \
         VALUES ($1, $2, $3, now() + interval '1 hour')",
    )
    .bind(&token)
    .bind(id)
    .bind(tenant_id)
    .execute(pool)
    .await
    .unwrap();
    (id, token)
}

fn cookie(token: &str) -> String {
    format!("arzi_session={token}")
}

#[sqlx::test(migrations = "./migrations")]
async fn creating_and_updating_a_user_leaves_audit_rows(pool: PgPool) -> sqlx::Result<()> {
    let tenant_id = seed_tenant(&pool).await;
    let (admin_id, admin_token) = seed_superuser(&pool, tenant_id).await;
    let router = app(AppState { pool: pool.clone() });

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
    let new_user_id: i64 = {
        let bytes = axum::body::to_bytes(create_resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let body: Value = serde_json::from_slice(&bytes).unwrap();
        body["id"].as_i64().unwrap()
    };

    router
        .clone()
        .oneshot(
            Request::post(format!("/api/v1/admin/users/{new_user_id}/disable"))
                .header(header::COOKIE, cookie(&admin_token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let rows: Vec<(String, Option<String>, i64)> = sqlx::query_as(
        "SELECT action, record_id, changed_by FROM audit_log \
         WHERE table_name = 'users' AND record_id = $1 ORDER BY changed_at",
    )
    .bind(new_user_id.to_string())
    .fetch_all(&pool)
    .await?;

    assert_eq!(rows.len(), 2, "expected an insert row and an update row");
    assert_eq!(rows[0].0, "insert");
    assert_eq!(rows[0].2, admin_id);
    assert_eq!(rows[1].0, "update");
    assert_eq!(rows[1].2, admin_id);

    let new_values: Value = sqlx::query_scalar(
        "SELECT new_values FROM audit_log WHERE table_name = 'users' AND record_id = $1 AND action = 'insert'",
    )
    .bind(new_user_id.to_string())
    .fetch_one(&pool)
    .await?;
    assert_eq!(new_values["username"], "newhire");

    Ok(())
}

#[sqlx::test(migrations = "./migrations")]
async fn failed_login_is_audited_without_the_password(pool: PgPool) -> sqlx::Result<()> {
    let tenant_id = seed_tenant(&pool).await;
    let router = app(AppState { pool: pool.clone() });

    sqlx::query("INSERT INTO users (tenant_id, username, password_hash) VALUES ($1, 'alice', 'x')")
        .bind(tenant_id)
        .execute(&pool)
        .await?;

    router
        .oneshot(
            Request::post("/api/v1/auth/login")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    r#"{"tenantSlug":"acme","username":"alice","password":"wrong-secret"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    let row: (String, Value) = sqlx::query_as(
        "SELECT action, new_values FROM audit_log WHERE table_name = 'auth_events' AND action = 'login_failed'",
    )
    .fetch_one(&pool)
    .await?;
    assert_eq!(row.0, "login_failed");
    assert_eq!(row.1["username"], "alice");
    assert!(
        !row.1.to_string().contains("wrong-secret"),
        "the audit row must never contain the attempted password"
    );

    Ok(())
}

#[sqlx::test(migrations = "./migrations")]
async fn permission_grant_is_audited_with_the_granting_admin(pool: PgPool) -> sqlx::Result<()> {
    let tenant_id = seed_tenant(&pool).await;
    let (admin_id, admin_token) = seed_superuser(&pool, tenant_id).await;
    let target_id: i64 = sqlx::query_scalar(
        "INSERT INTO users (tenant_id, username, password_hash) VALUES ($1, 'bob', 'x') RETURNING id",
    )
    .bind(tenant_id)
    .fetch_one(&pool)
    .await?;
    let router = app(AppState { pool: pool.clone() });

    router
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!("/api/v1/admin/users/{target_id}/permissions"))
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::COOKIE, cookie(&admin_token))
                .body(Body::from(r#"{"permissionIds":[1101]}"#))
                .unwrap(),
        )
        .await
        .unwrap();

    let row: (i64, Value) = sqlx::query_as(
        "SELECT changed_by, new_values FROM audit_log \
         WHERE table_name = 'auth_events' AND action = 'permission_granted' AND record_id = $1",
    )
    .bind(target_id.to_string())
    .fetch_one(&pool)
    .await?;
    assert_eq!(row.0, admin_id);
    assert_eq!(row.1["permissionIds"][0], 1101);

    Ok(())
}
