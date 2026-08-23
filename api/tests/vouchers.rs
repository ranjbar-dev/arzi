//! Automated version of step 2.3's manual test (docs/phase-2-accounting-
//! core.md §2.3): unbalanced draft saves fine, 0->1 rejected until balanced,
//! then succeeds; editing an issued voucher's lines is restricted; a date
//! outside the fiscal year is rejected; deleting a non-draft voucher is
//! rejected; a generated line is immutable from these endpoints.

use api::{app, AppState};
use axum::{
    body::Body,
    http::{header, Request, StatusCode},
};
use serde_json::Value;
use sqlx::PgPool;
use tower::ServiceExt;

struct Fixture {
    tenant_id: i64,
    fiscal_year_id: i64,
    account_id: i64, // a leaf account, postable
    token: String,
}

async fn seed(pool: &PgPool) -> Fixture {
    let tenant_id: i64 = sqlx::query_scalar("INSERT INTO tenants (slug, name) VALUES ('acme', 'Acme') RETURNING id")
        .fetch_one(pool)
        .await
        .unwrap();
    let user_id: i64 = sqlx::query_scalar(
        "INSERT INTO users (tenant_id, username, password_hash, is_superuser) \
         VALUES ($1, 'root', 'x', true) RETURNING id",
    )
    .bind(tenant_id)
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
    let fiscal_year_id: i64 = sqlx::query_scalar(
        "INSERT INTO fiscal_years (tenant_id, year, start_date, end_date) \
         VALUES ($1, 1403, '2024-03-20', '2025-03-20') RETURNING id",
    )
    .bind(tenant_id)
    .fetch_one(pool)
    .await
    .unwrap();
    let account_id: i64 = sqlx::query_scalar(
        "INSERT INTO accounts (tenant_id, general_ledger_code, subsidiary_code, name) \
         VALUES ($1, 1, 11, 'Cash') RETURNING id",
    )
    .bind(tenant_id)
    .fetch_one(pool)
    .await
    .unwrap();
    let account_id2: i64 = sqlx::query_scalar(
        "INSERT INTO accounts (tenant_id, general_ledger_code, subsidiary_code, name) \
         VALUES ($1, 2, 21, 'Revenue') RETURNING id",
    )
    .bind(tenant_id)
    .fetch_one(pool)
    .await
    .unwrap();
    let _ = account_id2;
    Fixture { tenant_id, fiscal_year_id, account_id, token }
}

fn cookie(token: &str) -> String {
    format!("arzi_session={token}")
}

async fn json_body(response: axum::response::Response) -> Value {
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

async fn create_voucher(router: &axum::Router, token: &str, fiscal_year_id: i64) -> i64 {
    let resp = router
        .clone()
        .oneshot(
            Request::post("/api/v1/vouchers")
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::COOKIE, cookie(token))
                .body(Body::from(
                    serde_json::json!({
                        "fiscalYearId": fiscal_year_id,
                        "voucherDate": "2024-04-01",
                        "description": "Test voucher"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    json_body(resp).await["id"].as_i64().unwrap()
}

async fn add_line(
    router: &axum::Router,
    token: &str,
    voucher_id: i64,
    account_id: i64,
    debit: i64,
    credit: i64,
) -> axum::response::Response {
    router
        .clone()
        .oneshot(
            Request::post(format!("/api/v1/vouchers/{voucher_id}/lines"))
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::COOKIE, cookie(token))
                .body(Body::from(
                    serde_json::json!({
                        "accountId": account_id,
                        "debit": debit,
                        "credit": credit,
                        "description": "line"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap()
}

#[sqlx::test(migrations = "./migrations")]
async fn draft_saves_unbalanced_then_balance_gates_confirmation(pool: PgPool) -> sqlx::Result<()> {
    let fx = seed(&pool).await;
    let router = app(AppState { pool: pool.clone() });

    let voucher_id = create_voucher(&router, &fx.token, fx.fiscal_year_id).await;

    // 1. Two unbalanced lines (debit 100, credit 50) -> saves fine as a draft.
    let l1 = add_line(&router, &fx.token, voucher_id, fx.account_id, 100, 0).await;
    assert_eq!(l1.status(), StatusCode::CREATED);
    let l2 = add_line(&router, &fx.token, voucher_id, fx.account_id, 0, 50).await;
    assert_eq!(l2.status(), StatusCode::CREATED);

    // 2. draft -> confirmed rejected: not balanced.
    let attempt = router
        .clone()
        .oneshot(
            Request::post(format!("/api/v1/vouchers/{voucher_id}/transition"))
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::COOKIE, cookie(&fx.token))
                .body(Body::from(r#"{"to":"confirmed"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(attempt.status(), StatusCode::BAD_REQUEST);
    let body = json_body(attempt).await;
    assert_eq!(body["error"], "voucher_not_balanced");

    // 3. Add a matching credit line -> debit == credit -> transition succeeds.
    let l3 = add_line(&router, &fx.token, voucher_id, fx.account_id, 0, 50).await;
    assert_eq!(l3.status(), StatusCode::CREATED);

    let confirm = router
        .clone()
        .oneshot(
            Request::post(format!("/api/v1/vouchers/{voucher_id}/transition"))
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::COOKIE, cookie(&fx.token))
                .body(Body::from(r#"{"to":"confirmed"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(confirm.status(), StatusCode::NO_CONTENT);

    // 4. Editing a line on the now-confirmed (non-draft) voucher is restricted.
    let get_resp = router
        .clone()
        .oneshot(
            Request::get(format!("/api/v1/vouchers/{voucher_id}"))
                .header(header::COOKIE, cookie(&fx.token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let detail = json_body(get_resp).await;
    assert_eq!(detail["status"], "confirmed");
    let line_id = detail["lines"][0]["id"].as_i64().unwrap();
    let edit_attempt = router
        .clone()
        .oneshot(
            Request::put(format!("/api/v1/vouchers/{voucher_id}/lines/{line_id}"))
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::COOKIE, cookie(&fx.token))
                .body(Body::from(
                    serde_json::json!({ "accountId": fx.account_id, "debit": 999, "credit": 0, "description": "x" })
                        .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(edit_attempt.status(), StatusCode::BAD_REQUEST);
    let body = json_body(edit_attempt).await;
    assert_eq!(body["error"], "not_draft");

    // 6. Deleting a non-draft (confirmed) voucher is rejected.
    let delete_attempt = router
        .clone()
        .oneshot(
            Request::delete(format!("/api/v1/vouchers/{voucher_id}"))
                .header(header::COOKIE, cookie(&fx.token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(delete_attempt.status(), StatusCode::BAD_REQUEST);

    let _ = fx.tenant_id;
    Ok(())
}

#[sqlx::test(migrations = "./migrations")]
async fn voucher_date_outside_fiscal_year_is_rejected(pool: PgPool) -> sqlx::Result<()> {
    let fx = seed(&pool).await;
    let router = app(AppState { pool: pool.clone() });

    let resp = router
        .oneshot(
            Request::post("/api/v1/vouchers")
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::COOKIE, cookie(&fx.token))
                .body(Body::from(
                    serde_json::json!({
                        "fiscalYearId": fx.fiscal_year_id,
                        "voucherDate": "2026-01-01",
                        "description": "Out of range"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    let body = json_body(resp).await;
    assert_eq!(body["error"], "date_outside_fiscal_year");

    Ok(())
}

#[sqlx::test(migrations = "./migrations")]
async fn generated_line_is_immutable_from_the_voucher_editor(pool: PgPool) -> sqlx::Result<()> {
    let fx = seed(&pool).await;
    let router = app(AppState { pool: pool.clone() });

    let voucher_id = create_voucher(&router, &fx.token, fx.fiscal_year_id).await;
    // Simulate a line generated by another domain (no domain posts automatically yet — 2.3's
    // manual test #7 says to simulate it directly).
    let line_id: i64 = sqlx::query_scalar(
        "INSERT INTO voucher_lines \
         (tenant_id, voucher_id, fiscal_year_id, line_date, debit_amount, credit_amount, \
          description, account_id, source_module) \
         VALUES ($1, $2, $3, '2024-04-01', 100, 0, 'generated', $4, 1) RETURNING id",
    )
    .bind(fx.tenant_id)
    .bind(voucher_id)
    .bind(fx.fiscal_year_id)
    .bind(fx.account_id)
    .fetch_one(&pool)
    .await?;

    let edit_attempt = router
        .clone()
        .oneshot(
            Request::put(format!("/api/v1/vouchers/{voucher_id}/lines/{line_id}"))
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::COOKIE, cookie(&fx.token))
                .body(Body::from(
                    serde_json::json!({ "accountId": fx.account_id, "debit": 1, "credit": 0, "description": "x" })
                        .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(edit_attempt.status(), StatusCode::FORBIDDEN);

    let delete_attempt = router
        .oneshot(
            Request::delete(format!("/api/v1/vouchers/{voucher_id}/lines/{line_id}"))
                .header(header::COOKIE, cookie(&fx.token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(delete_attempt.status(), StatusCode::FORBIDDEN);

    Ok(())
}

#[sqlx::test(migrations = "./migrations")]
async fn non_leaf_account_and_empty_voucher_are_rejected(pool: PgPool) -> sqlx::Result<()> {
    let fx = seed(&pool).await;
    let router = app(AppState { pool: pool.clone() });

    let parent_id: i64 = sqlx::query_scalar(
        "INSERT INTO accounts (tenant_id, general_ledger_code, name, child_count) \
         VALUES ($1, 9, 'Non-leaf', 1) RETURNING id",
    )
    .bind(fx.tenant_id)
    .fetch_one(&pool)
    .await?;

    let voucher_id = create_voucher(&router, &fx.token, fx.fiscal_year_id).await;

    let non_leaf = add_line(&router, &fx.token, voucher_id, parent_id, 10, 0).await;
    assert_eq!(non_leaf.status(), StatusCode::BAD_REQUEST);
    let body = json_body(non_leaf).await;
    assert_eq!(body["error"], "account_not_leaf");

    // Empty voucher (no lines) -> confirmation rejected.
    let confirm = router
        .oneshot(
            Request::post(format!("/api/v1/vouchers/{voucher_id}/transition"))
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::COOKIE, cookie(&fx.token))
                .body(Body::from(r#"{"to":"confirmed"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(confirm.status(), StatusCode::BAD_REQUEST);
    let body = json_body(confirm).await;
    assert_eq!(body["error"], "voucher_empty");

    Ok(())
}

/// Regression test: `?fiscalYearId=` must actually filter — a previous
/// version's `ListQuery` had no `rename_all = "camelCase"`, so serde looked
/// for a query param literally named `fiscal_year_id` and silently found
/// nothing, making the filter a no-op for every camelCase caller (which is
/// all of them — the frontend, and every curl example in this repo).
#[sqlx::test(migrations = "./migrations")]
async fn list_is_actually_filtered_by_fiscal_year(pool: PgPool) -> sqlx::Result<()> {
    let fx = seed(&pool).await;
    let other_fiscal_year_id: i64 = sqlx::query_scalar(
        "INSERT INTO fiscal_years (tenant_id, year, start_date, end_date) \
         VALUES ($1, 1404, '2025-03-21', '2026-03-20') RETURNING id",
    )
    .bind(fx.tenant_id)
    .fetch_one(&pool)
    .await?;
    let router = app(AppState { pool: pool.clone() });

    let voucher_id = create_voucher(&router, &fx.token, fx.fiscal_year_id).await;
    let _ = add_line(&router, &fx.token, voucher_id, fx.account_id, 10, 0).await;

    let matching = router
        .clone()
        .oneshot(
            Request::get(format!("/api/v1/vouchers?fiscalYearId={}", fx.fiscal_year_id))
                .header(header::COOKIE, cookie(&fx.token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let matching_body = json_body(matching).await;
    assert_eq!(matching_body.as_array().unwrap().len(), 1);

    let other = router
        .oneshot(
            Request::get(format!("/api/v1/vouchers?fiscalYearId={other_fiscal_year_id}"))
                .header(header::COOKIE, cookie(&fx.token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let other_body = json_body(other).await;
    assert_eq!(other_body.as_array().unwrap().len(), 0);

    Ok(())
}
