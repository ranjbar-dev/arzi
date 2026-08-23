//! Automated test for the issued-cheque payment batch (A9 unblocked as
//! "batch" per explicit user decision, docs/phase-4-treasury.md §4.5
//! follow-up): N payee lines -> N debit lines + one credit line to the bank
//! account, no orphaned rows on delete.

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
    bank_account_id: i64,
    payee_account_ids: [i64; 2],
    token: String,
}

async fn seed(pool: &PgPool) -> Fixture {
    let tenant_id: i64 =
        sqlx::query_scalar("INSERT INTO tenants (slug, name) VALUES ('acme', 'Acme') RETURNING id")
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
    let bank_account_id = leaf(101, 1, "Bank").await;
    let payee_account_ids = [leaf(301, 1, "Payee A").await, leaf(301, 2, "Payee B").await];
    Fixture {
        fiscal_year_id,
        bank_account_id,
        payee_account_ids,
        token,
    }
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

async fn post(
    router: &axum::Router,
    token: &str,
    path: &str,
    body: Value,
) -> axum::response::Response {
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
        .oneshot(
            Request::get(path)
                .header(header::COOKIE, cookie(token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap()
}

async fn delete(router: &axum::Router, token: &str, path: &str) -> axum::response::Response {
    router
        .clone()
        .oneshot(
            Request::delete(path)
                .header(header::COOKIE, cookie(token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap()
}

#[sqlx::test(migrations = "./migrations")]
async fn two_payees_post_two_debits_one_credit(pool: PgPool) -> sqlx::Result<()> {
    let fx = seed(&pool).await;
    let router = app(AppState { pool: pool.clone() });

    let create = post(
        &router,
        &fx.token,
        "/api/v1/cheque-payment-batches",
        json!({
            "fiscalYearId": fx.fiscal_year_id,
            "batchNumber": "B-1",
            "issueDate": "2018-04-01",
            "description": "Weekly payment run",
            "bankAccountId": fx.bank_account_id,
            "lines": [
                { "payeeAccountId": fx.payee_account_ids[0], "amount": 100_000, "description": "Invoice 1" },
                { "payeeAccountId": fx.payee_account_ids[1], "amount": 250_000, "description": "Invoice 2" },
            ],
        }),
    )
    .await;
    assert_eq!(create.status(), StatusCode::CREATED);
    let batch_id = json_body(create).await["id"].as_i64().unwrap();

    let detail = json_body(
        get(
            &router,
            &fx.token,
            &format!("/api/v1/cheque-payment-batches/{batch_id}"),
        )
        .await,
    )
    .await;
    assert_eq!(detail["totalAmount"], 350_000);
    assert_eq!(detail["lineCount"], 2);

    let voucher_id = detail["voucherId"].as_i64().unwrap();
    let (total_debit, total_credit, line_count): (i64, i64, i32) =
        sqlx::query_as("SELECT total_debit, total_credit, line_count FROM vouchers WHERE id = $1")
            .bind(voucher_id)
            .fetch_one(&pool)
            .await?;
    assert_eq!(total_debit, total_credit);
    assert_eq!(total_debit, 350_000);
    assert_eq!(line_count, 3, "2 payee debit lines + 1 bank credit line");

    let credit_row: (i64, i64) = sqlx::query_as(
        "SELECT account_id, credit_amount FROM voucher_lines WHERE voucher_id = $1 AND credit_amount > 0",
    )
    .bind(voucher_id)
    .fetch_one(&pool)
    .await?;
    assert_eq!(credit_row.0, fx.bank_account_id);
    assert_eq!(credit_row.1, 350_000);

    Ok(())
}

#[sqlx::test(migrations = "./migrations")]
async fn delete_removes_batch_lines_and_voucher_cleanly(pool: PgPool) -> sqlx::Result<()> {
    let fx = seed(&pool).await;
    let router = app(AppState { pool: pool.clone() });

    let create = post(
        &router,
        &fx.token,
        "/api/v1/cheque-payment-batches",
        json!({
            "fiscalYearId": fx.fiscal_year_id,
            "issueDate": "2018-04-01",
            "description": "Single payee run",
            "bankAccountId": fx.bank_account_id,
            "lines": [{ "payeeAccountId": fx.payee_account_ids[0], "amount": 50_000 }],
        }),
    )
    .await;
    let batch_id = json_body(create).await["id"].as_i64().unwrap();
    let voucher_id = json_body(
        get(
            &router,
            &fx.token,
            &format!("/api/v1/cheque-payment-batches/{batch_id}"),
        )
        .await,
    )
    .await["voucherId"]
        .as_i64()
        .unwrap();

    let del = delete(
        &router,
        &fx.token,
        &format!("/api/v1/cheque-payment-batches/{batch_id}"),
    )
    .await;
    assert_eq!(del.status(), StatusCode::NO_CONTENT);

    let after = get(
        &router,
        &fx.token,
        &format!("/api/v1/cheque-payment-batches/{batch_id}"),
    )
    .await;
    assert_eq!(after.status(), StatusCode::NOT_FOUND);
    let voucher_gone: Option<i64> = sqlx::query_scalar("SELECT id FROM vouchers WHERE id = $1")
        .bind(voucher_id)
        .fetch_optional(&pool)
        .await?;
    assert!(voucher_gone.is_none());
    let lines_gone: i64 =
        sqlx::query_scalar("SELECT count(*) FROM cheque_payment_batch_lines WHERE batch_id = $1")
            .bind(batch_id)
            .fetch_one(&pool)
            .await?;
    assert_eq!(lines_gone, 0);

    Ok(())
}

#[sqlx::test(migrations = "./migrations")]
async fn requires_description_and_at_least_one_line(pool: PgPool) -> sqlx::Result<()> {
    let fx = seed(&pool).await;
    let router = app(AppState { pool: pool.clone() });

    let blank_desc = post(
        &router,
        &fx.token,
        "/api/v1/cheque-payment-batches",
        json!({
            "fiscalYearId": fx.fiscal_year_id, "issueDate": "2018-04-01", "description": "   ",
            "bankAccountId": fx.bank_account_id,
            "lines": [{ "payeeAccountId": fx.payee_account_ids[0], "amount": 1000 }],
        }),
    )
    .await;
    assert_eq!(blank_desc.status(), StatusCode::BAD_REQUEST);
    assert_eq!(json_body(blank_desc).await["error"], "description_required");

    let empty_lines = post(
        &router,
        &fx.token,
        "/api/v1/cheque-payment-batches",
        json!({
            "fiscalYearId": fx.fiscal_year_id, "issueDate": "2018-04-01", "description": "x",
            "bankAccountId": fx.bank_account_id, "lines": [],
        }),
    )
    .await;
    assert_eq!(empty_lines.status(), StatusCode::BAD_REQUEST);
    assert_eq!(
        json_body(empty_lines).await["error"],
        "at_least_one_line_required"
    );

    Ok(())
}
