//! Automated version of step 4.4's manual test #2 and #3 (docs/phase-4-
//! treasury.md §4.4) for petty-cash claims: three expense lines -> three
//! debit lines + one credit line, correct total, no "N persons" narration
//! bug, and a clean delete.

use api::{app, AppState};
use axum::{
    body::Body,
    http::{header, Request, StatusCode},
};
use serde_json::{json, Value};
use sqlx::PgPool;
use tower::ServiceExt;

struct Fixture {
    fiscal_year_id: i64,
    custodian_account_id: i64,
    expense_account_ids: [i64; 3],
    token: String,
}

async fn seed(pool: &PgPool) -> Fixture {
    let tenant_id: i64 = sqlx::query_scalar("INSERT INTO tenants (slug, name) VALUES ('acme', 'Acme') RETURNING id")
        .fetch_one(pool)
        .await
        .unwrap();
    let user_id: i64 = sqlx::query_scalar(
        "INSERT INTO users (tenant_id, username, password_hash, is_superuser) VALUES ($1, 'root', 'x', true) RETURNING id",
    )
    .bind(tenant_id)
    .fetch_one(pool)
    .await
    .unwrap();
    let token = format!("test-session-{user_id}");
    sqlx::query("INSERT INTO sessions (id, user_id, tenant_id, expires_at) VALUES ($1, $2, $3, now() + interval '1 hour')")
        .bind(&token)
        .bind(user_id)
        .bind(tenant_id)
        .execute(pool)
        .await
        .unwrap();
    let fiscal_year_id: i64 = sqlx::query_scalar(
        "INSERT INTO fiscal_years (tenant_id, year, start_date, end_date) VALUES ($1, 1397, '2018-03-21', '2019-03-20') RETURNING id",
    )
    .bind(tenant_id)
    .fetch_one(pool)
    .await
    .unwrap();
    let leaf = |gl: i32, sub: i32, name: &'static str| {
        let pool = pool.clone();
        async move {
            let id: i64 = sqlx::query_scalar(
                "INSERT INTO accounts (tenant_id, general_ledger_code, subsidiary_code, name) VALUES ($1, $2, $3, $4) RETURNING id",
            )
            .bind(tenant_id)
            .bind(gl)
            .bind(sub)
            .bind(name)
            .fetch_one(&pool)
            .await
            .unwrap();
            id
        }
    };
    let custodian_account_id = leaf(102, 1, "Custodian").await;
    let expense_account_ids = [leaf(601, 1, "Stationery").await, leaf(601, 2, "Transport").await, leaf(601, 3, "Postage").await];
    Fixture { fiscal_year_id, custodian_account_id, expense_account_ids, token }
}

fn cookie(token: &str) -> String {
    format!("arzi_session={token}")
}

async fn json_body(response: axum::response::Response) -> Value {
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

async fn post(router: &axum::Router, token: &str, path: &str, body: Value) -> axum::response::Response {
    router
        .clone()
        .oneshot(
            Request::post(path)
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::COOKIE, cookie(token))
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap()
}

async fn get(router: &axum::Router, token: &str, path: &str) -> axum::response::Response {
    router
        .clone()
        .oneshot(Request::get(path).header(header::COOKIE, cookie(token)).body(Body::empty()).unwrap())
        .await
        .unwrap()
}

async fn delete(router: &axum::Router, token: &str, path: &str) -> axum::response::Response {
    router
        .clone()
        .oneshot(Request::delete(path).header(header::COOKIE, cookie(token)).body(Body::empty()).unwrap())
        .await
        .unwrap()
}

#[sqlx::test(migrations = "./migrations")]
async fn three_lines_post_three_debits_one_credit(pool: PgPool) -> sqlx::Result<()> {
    let fx = seed(&pool).await;
    let router = app(AppState { pool: pool.clone() });

    let create = post(
        &router,
        &fx.token,
        "/api/v1/petty-cash-claims",
        json!({
            "fiscalYearId": fx.fiscal_year_id,
            "claimNumber": "PC-1",
            "claimDate": "2018-04-01",
            "custodianAccountId": fx.custodian_account_id,
            "lines": [
                { "expenseAccountId": fx.expense_account_ids[0], "amount": 50_000, "description": "Stationery" },
                { "expenseAccountId": fx.expense_account_ids[1], "amount": 120_000, "description": "Taxi fare" },
                { "expenseAccountId": fx.expense_account_ids[2], "amount": 30_000, "description": "Postage" },
            ],
        }),
    )
    .await;
    assert_eq!(create.status(), StatusCode::CREATED);
    let claim_id = json_body(create).await["id"].as_i64().unwrap();

    let detail = json_body(get(&router, &fx.token, &format!("/api/v1/petty-cash-claims/{claim_id}")).await).await;
    assert_eq!(detail["totalAmount"], 200_000, "total must be the sum of the lines, not entered");
    assert_eq!(detail["lineCount"], 3);
    assert_eq!(detail["lines"].as_array().unwrap().len(), 3);

    let voucher_id = detail["voucherId"].as_i64().unwrap();
    let (total_debit, total_credit, line_count): (i64, i64, i32) =
        sqlx::query_as("SELECT total_debit, total_credit, line_count FROM vouchers WHERE id = $1")
            .bind(voucher_id)
            .fetch_one(&pool)
            .await?;
    assert_eq!(total_debit, total_credit);
    assert_eq!(total_debit, 200_000);
    assert_eq!(line_count, 4, "3 expense debit lines + 1 custodian credit line");

    let debit_count: i64 = sqlx::query_scalar("SELECT count(*) FROM voucher_lines WHERE voucher_id = $1 AND debit_amount > 0")
        .bind(voucher_id)
        .fetch_one(&pool)
        .await?;
    assert_eq!(debit_count, 3);
    let credit_row: (i64, i64) = sqlx::query_as(
        "SELECT account_id, credit_amount FROM voucher_lines WHERE voucher_id = $1 AND credit_amount > 0",
    )
    .bind(voucher_id)
    .fetch_one(&pool)
    .await?;
    assert_eq!(credit_row.0, fx.custodian_account_id);
    assert_eq!(credit_row.1, 200_000);

    // The "N persons" narration bug must NOT be reproduced.
    let credit_narration: Option<String> = sqlx::query_scalar(
        "SELECT description FROM voucher_lines WHERE voucher_id = $1 AND credit_amount > 0",
    )
    .bind(voucher_id)
    .fetch_one(&pool)
    .await?;
    let narration = credit_narration.unwrap_or_default();
    assert!(!narration.contains("persons") && !narration.contains("نفر"), "must not say '3 persons': {narration}");
    assert!(narration.contains("expense line"), "narration should describe expense lines: {narration}");

    Ok(())
}

#[sqlx::test(migrations = "./migrations")]
async fn delete_removes_claim_lines_and_voucher_cleanly(pool: PgPool) -> sqlx::Result<()> {
    let fx = seed(&pool).await;
    let router = app(AppState { pool: pool.clone() });

    let create = post(
        &router,
        &fx.token,
        "/api/v1/petty-cash-claims",
        json!({
            "fiscalYearId": fx.fiscal_year_id,
            "claimDate": "2018-04-01",
            "custodianAccountId": fx.custodian_account_id,
            "lines": [{ "expenseAccountId": fx.expense_account_ids[0], "amount": 15_000 }],
        }),
    )
    .await;
    let claim_id = json_body(create).await["id"].as_i64().unwrap();
    let voucher_id =
        json_body(get(&router, &fx.token, &format!("/api/v1/petty-cash-claims/{claim_id}")).await).await["voucherId"]
            .as_i64()
            .unwrap();

    let del = delete(&router, &fx.token, &format!("/api/v1/petty-cash-claims/{claim_id}")).await;
    assert_eq!(del.status(), StatusCode::NO_CONTENT);

    let after = get(&router, &fx.token, &format!("/api/v1/petty-cash-claims/{claim_id}")).await;
    assert_eq!(after.status(), StatusCode::NOT_FOUND);
    let voucher_gone: Option<i64> = sqlx::query_scalar("SELECT id FROM vouchers WHERE id = $1")
        .bind(voucher_id)
        .fetch_optional(&pool)
        .await?;
    assert!(voucher_gone.is_none());
    let claim_lines_gone: i64 = sqlx::query_scalar("SELECT count(*) FROM petty_cash_claim_lines WHERE claim_id = $1")
        .bind(claim_id)
        .fetch_one(&pool)
        .await?;
    assert_eq!(claim_lines_gone, 0);
    let voucher_lines_gone: i64 = sqlx::query_scalar("SELECT count(*) FROM voucher_lines WHERE voucher_id = $1")
        .bind(voucher_id)
        .fetch_one(&pool)
        .await?;
    assert_eq!(voucher_lines_gone, 0);

    Ok(())
}

#[sqlx::test(migrations = "./migrations")]
async fn at_least_one_line_required(pool: PgPool) -> sqlx::Result<()> {
    let fx = seed(&pool).await;
    let router = app(AppState { pool: pool.clone() });

    let empty = post(
        &router,
        &fx.token,
        "/api/v1/petty-cash-claims",
        json!({
            "fiscalYearId": fx.fiscal_year_id,
            "claimDate": "2018-04-01",
            "custodianAccountId": fx.custodian_account_id,
            "lines": [],
        }),
    )
    .await;
    assert_eq!(empty.status(), StatusCode::BAD_REQUEST);
    assert_eq!(json_body(empty).await["error"], "at_least_one_line_required");

    Ok(())
}
