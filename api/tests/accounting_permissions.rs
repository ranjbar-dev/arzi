//! Automated version of step 2.8's manual test (docs/phase-2-accounting-
//! core.md §2.8): a session with the matching permission succeeds, one
//! without it gets 403 -- spot-checked across accounts.rs and vouchers.rs's
//! routes, with create-account/amend-account (the fixed 1102/1103 legacy
//! collision) specifically verified as independently grantable.

use api::{app, AppState};
use axum::{
    body::Body,
    http::{header, Request, StatusCode},
};
use serde_json::{json, Value};
use sqlx::PgPool;
use tower::ServiceExt;

async fn make_user(pool: &PgPool, tenant_id: i64, username: &str) -> (i64, String) {
    let user_id: i64 = sqlx::query_scalar(
        "INSERT INTO users (tenant_id, username, password_hash, is_superuser) \
         VALUES ($1, $2, 'x', false) RETURNING id",
    )
    .bind(tenant_id)
    .bind(username)
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
    (user_id, token)
}

async fn grant(pool: &PgPool, tenant_id: i64, user_id: i64, code: &str) {
    sqlx::query(
        "INSERT INTO user_permissions (tenant_id, user_id, permission_id) \
         SELECT $1, $2, id FROM permissions WHERE code = $3",
    )
    .bind(tenant_id)
    .bind(user_id)
    .bind(code)
    .execute(pool)
    .await
    .unwrap();
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
    router
        .clone()
        .oneshot(
            Request::builder()
                .method(method)
                .uri(path)
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::COOKIE, cookie(token))
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap()
}

#[sqlx::test(migrations = "./migrations")]
async fn create_account_and_amend_account_are_independently_grantable(
    pool: PgPool,
) -> sqlx::Result<()> {
    let tenant_id: i64 =
        sqlx::query_scalar("INSERT INTO tenants (slug, name) VALUES ('acme', 'Acme') RETURNING id")
            .fetch_one(&pool)
            .await
            .unwrap();
    let (user_id, token) = make_user(&pool, tenant_id, "clerk").await;
    let router = app(AppState { pool: pool.clone() });

    // No permissions at all -> every account route 403s.
    let create = req(
        &router,
        "POST",
        "/api/v1/accounts",
        &token,
        json!({ "code": 1, "name": "Assets" }),
    )
    .await;
    assert_eq!(create.status(), StatusCode::FORBIDDEN);
    let list = req(&router, "GET", "/api/v1/accounts", &token, Value::Null).await;
    assert_eq!(list.status(), StatusCode::FORBIDDEN);

    // Grant ONLY create_account (1102) -> create succeeds, but amend/delete on the
    // resulting node still 403 (the fixed 1102/1103 collision -- legacy conflated these).
    grant(&pool, tenant_id, user_id, "create_account").await;
    let create = req(
        &router,
        "POST",
        "/api/v1/accounts",
        &token,
        json!({ "code": 1, "name": "Assets" }),
    )
    .await;
    assert_eq!(create.status(), StatusCode::CREATED);
    let account_id = json_body(create).await["id"].as_i64().unwrap();

    let rename = req(
        &router,
        "PUT",
        &format!("/api/v1/accounts/{account_id}/name"),
        &token,
        json!({ "name": "Assets 2" }),
    )
    .await;
    assert_eq!(rename.status(), StatusCode::FORBIDDEN);
    let delete = req(
        &router,
        "DELETE",
        &format!("/api/v1/accounts/{account_id}"),
        &token,
        Value::Null,
    )
    .await;
    assert_eq!(delete.status(), StatusCode::FORBIDDEN);

    // Grant amend_account (1103) too -> rename now succeeds, but a second create still
    // needs 1102 independently (it does, already granted) and delete still needs 1104.
    grant(&pool, tenant_id, user_id, "amend_account").await;
    let rename = req(
        &router,
        "PUT",
        &format!("/api/v1/accounts/{account_id}/name"),
        &token,
        json!({ "name": "Assets 2" }),
    )
    .await;
    assert_eq!(rename.status(), StatusCode::NO_CONTENT);
    let delete = req(
        &router,
        "DELETE",
        &format!("/api/v1/accounts/{account_id}"),
        &token,
        Value::Null,
    )
    .await;
    assert_eq!(delete.status(), StatusCode::FORBIDDEN);

    // Grant delete_account (1104) -> delete now succeeds too. All three fully independent.
    grant(&pool, tenant_id, user_id, "delete_account").await;
    let delete = req(
        &router,
        "DELETE",
        &format!("/api/v1/accounts/{account_id}"),
        &token,
        Value::Null,
    )
    .await;
    assert_eq!(delete.status(), StatusCode::NO_CONTENT);

    // account_list (1101) was never granted -- confirms it's a real, separate gate too.
    let list = req(&router, "GET", "/api/v1/accounts", &token, Value::Null).await;
    assert_eq!(list.status(), StatusCode::FORBIDDEN);
    grant(&pool, tenant_id, user_id, "account_list").await;
    let list = req(&router, "GET", "/api/v1/accounts", &token, Value::Null).await;
    assert_eq!(list.status(), StatusCode::OK);

    Ok(())
}

#[sqlx::test(migrations = "./migrations")]
async fn voucher_routes_are_gated_by_their_catalogue_ids(pool: PgPool) -> sqlx::Result<()> {
    let tenant_id: i64 =
        sqlx::query_scalar("INSERT INTO tenants (slug, name) VALUES ('acme', 'Acme') RETURNING id")
            .fetch_one(&pool)
            .await
            .unwrap();
    let (user_id, token) = make_user(&pool, tenant_id, "clerk").await;
    let router = app(AppState { pool: pool.clone() });

    let fiscal_year_id: i64 = sqlx::query_scalar(
        "INSERT INTO fiscal_years (tenant_id, year, start_date, end_date) \
         VALUES ($1, 1403, '2024-03-20', '2025-03-20') RETURNING id",
    )
    .bind(tenant_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    sqlx::query("INSERT INTO accounts (tenant_id, general_ledger_code, name) VALUES ($1, 1, 'Assets (Kol)')")
        .bind(tenant_id)
        .execute(&pool)
        .await
        .unwrap();
    let cash_id: i64 = sqlx::query_scalar(
        "INSERT INTO accounts (tenant_id, general_ledger_code, subsidiary_code, name) \
         VALUES ($1, 1, 11, 'Cash') RETURNING id",
    )
    .bind(tenant_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    let bank_id: i64 = sqlx::query_scalar(
        "INSERT INTO accounts (tenant_id, general_ledger_code, subsidiary_code, name) \
         VALUES ($1, 1, 12, 'Bank') RETURNING id",
    )
    .bind(tenant_id)
    .fetch_one(&pool)
    .await
    .unwrap();

    // list -- no view permission at all -> 403.
    let list = req(&router, "GET", "/api/v1/vouchers", &token, Value::Null).await;
    assert_eq!(list.status(), StatusCode::FORBIDDEN);
    grant(&pool, tenant_id, user_id, "list_draft_subsidiary_documents").await;
    let list = req(&router, "GET", "/api/v1/vouchers", &token, Value::Null).await;
    assert_eq!(list.status(), StatusCode::OK);

    // create -- needs post_subsidiary_document (1113), not the view permission just granted.
    let create = req(
        &router,
        "POST",
        "/api/v1/vouchers",
        &token,
        json!({ "fiscalYearId": fiscal_year_id, "voucherDate": "2024-04-01", "description": "d" }),
    )
    .await;
    assert_eq!(create.status(), StatusCode::FORBIDDEN);
    grant(&pool, tenant_id, user_id, "post_subsidiary_document").await;
    let create = req(
        &router,
        "POST",
        "/api/v1/vouchers",
        &token,
        json!({ "fiscalYearId": fiscal_year_id, "voucherDate": "2024-04-01", "description": "d" }),
    )
    .await;
    assert_eq!(create.status(), StatusCode::CREATED);
    let voucher_id = json_body(create).await["id"].as_i64().unwrap();

    // add_line -- needs amend_subsidiary_document (1114), independent of post/view.
    let add_line = req(
        &router,
        "POST",
        &format!("/api/v1/vouchers/{voucher_id}/lines"),
        &token,
        json!({ "accountId": cash_id, "debit": 100, "credit": 0, "description": "l" }),
    )
    .await;
    assert_eq!(add_line.status(), StatusCode::FORBIDDEN);
    grant(&pool, tenant_id, user_id, "amend_subsidiary_document").await;
    for (account, debit, credit) in [(cash_id, 100, 0), (bank_id, 0, 100)] {
        let add_line = req(
            &router,
            "POST",
            &format!("/api/v1/vouchers/{voucher_id}/lines"),
            &token,
            json!({ "accountId": account, "debit": debit, "credit": credit, "description": "l" }),
        )
        .await;
        assert_eq!(add_line.status(), StatusCode::CREATED);
    }

    // transition draft->confirmed -- needs approve_subsidiary_document (1116), not amend.
    let confirm = req(
        &router,
        "POST",
        &format!("/api/v1/vouchers/{voucher_id}/transition"),
        &token,
        json!({ "to": "confirmed" }),
    )
    .await;
    assert_eq!(confirm.status(), StatusCode::FORBIDDEN);
    grant(&pool, tenant_id, user_id, "approve_subsidiary_document").await;
    let confirm = req(
        &router,
        "POST",
        &format!("/api/v1/vouchers/{voucher_id}/transition"),
        &token,
        json!({ "to": "confirmed" }),
    )
    .await;
    assert_eq!(confirm.status(), StatusCode::NO_CONTENT);

    // confirmed->posted -- needs the DIFFERENT id post_subsidiary_document_permanently (1117),
    // proving the four transition directions really are independently gated.
    let post = req(
        &router,
        "POST",
        &format!("/api/v1/vouchers/{voucher_id}/transition"),
        &token,
        json!({ "to": "posted" }),
    )
    .await;
    assert_eq!(post.status(), StatusCode::FORBIDDEN);
    grant(
        &pool,
        tenant_id,
        user_id,
        "post_subsidiary_document_permanently",
    )
    .await;
    let post = req(
        &router,
        "POST",
        &format!("/api/v1/vouchers/{voucher_id}/transition"),
        &token,
        json!({ "to": "posted" }),
    )
    .await;
    assert_eq!(post.status(), StatusCode::NO_CONTENT);

    // lock -- needs lock_subsidiary_document (1144).
    let lock = req(
        &router,
        "POST",
        &format!("/api/v1/vouchers/{voucher_id}/lock"),
        &token,
        Value::Null,
    )
    .await;
    assert_eq!(lock.status(), StatusCode::FORBIDDEN);
    grant(&pool, tenant_id, user_id, "lock_subsidiary_document").await;
    let lock = req(
        &router,
        "POST",
        &format!("/api/v1/vouchers/{voucher_id}/lock"),
        &token,
        Value::Null,
    )
    .await;
    assert_eq!(lock.status(), StatusCode::NO_CONTENT);

    Ok(())
}
