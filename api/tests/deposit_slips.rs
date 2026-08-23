//! Automated version of step 4.4's manual test #1 and #3 (docs/phase-4-
//! treasury.md §4.4) for deposit slips: correct (unswapped) narration on
//! both lines, and a clean delete.

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
    payer_account_id: i64,
    bank_account_id: i64,
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
    let payer_account_id = leaf(103, 1, "Payer").await;
    let bank_account_id = leaf(101, 1, "Bank").await;
    Fixture { fiscal_year_id, payer_account_id, bank_account_id, token }
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
async fn posts_correctly_narrated_balanced_voucher(pool: PgPool) -> sqlx::Result<()> {
    let fx = seed(&pool).await;
    let router = app(AppState { pool: pool.clone() });

    let create = post(
        &router,
        &fx.token,
        "/api/v1/deposit-slips",
        json!({
            "fiscalYearId": fx.fiscal_year_id,
            "slipNumber": "SLIP-1",
            "slipDate": "2018-04-01",
            "amount": 400_000,
            "description": "Cash deposit from customer",
            "payerAccountId": fx.payer_account_id,
            "bankAccountId": fx.bank_account_id,
            "channel": "cash_slip",
        }),
    )
    .await;
    assert_eq!(create.status(), StatusCode::CREATED);
    let slip_id = json_body(create).await["id"].as_i64().unwrap();

    let detail = json_body(get(&router, &fx.token, &format!("/api/v1/deposit-slips/{slip_id}")).await).await;
    let voucher_id = detail["voucherId"].as_i64().expect("slip must have a real voucher (B8.5-defect-6 fix)");

    let (total_debit, total_credit): (i64, i64) =
        sqlx::query_as("SELECT total_debit, total_credit FROM vouchers WHERE id = $1")
            .bind(voucher_id)
            .fetch_one(&pool)
            .await?;
    assert_eq!(total_debit, total_credit);
    assert_eq!(total_debit, 400_000);

    // Debit line must be the BANK account, credit line the PAYER -- and
    // both must carry the SAME (correct) narration, not two swapped ones.
    let debit_row: (i64, Option<String>) = sqlx::query_as(
        "SELECT account_id, description FROM voucher_lines WHERE voucher_id = $1 AND debit_amount > 0",
    )
    .bind(voucher_id)
    .fetch_one(&pool)
    .await?;
    let credit_row: (i64, Option<String>) = sqlx::query_as(
        "SELECT account_id, description FROM voucher_lines WHERE voucher_id = $1 AND credit_amount > 0",
    )
    .bind(voucher_id)
    .fetch_one(&pool)
    .await?;
    assert_eq!(debit_row.0, fx.bank_account_id, "debit line must be the bank account");
    assert_eq!(credit_row.0, fx.payer_account_id, "credit line must be the payer");
    assert_eq!(debit_row.1, credit_row.1, "narration must be identical on both lines -- no swap possible");
    assert_eq!(debit_row.1.as_deref(), Some("Cash deposit from customer"));

    Ok(())
}

#[sqlx::test(migrations = "./migrations")]
async fn delete_removes_slip_and_voucher_cleanly(pool: PgPool) -> sqlx::Result<()> {
    let fx = seed(&pool).await;
    let router = app(AppState { pool: pool.clone() });

    let create = post(
        &router,
        &fx.token,
        "/api/v1/deposit-slips",
        json!({
            "fiscalYearId": fx.fiscal_year_id,
            "slipDate": "2018-04-01",
            "amount": 100_000,
            "payerAccountId": fx.payer_account_id,
            "bankAccountId": fx.bank_account_id,
            "channel": "pos_terminal",
        }),
    )
    .await;
    let slip_id = json_body(create).await["id"].as_i64().unwrap();
    let voucher_id = json_body(get(&router, &fx.token, &format!("/api/v1/deposit-slips/{slip_id}")).await).await["voucherId"]
        .as_i64()
        .unwrap();

    let del = delete(&router, &fx.token, &format!("/api/v1/deposit-slips/{slip_id}")).await;
    assert_eq!(del.status(), StatusCode::NO_CONTENT);

    let after = get(&router, &fx.token, &format!("/api/v1/deposit-slips/{slip_id}")).await;
    assert_eq!(after.status(), StatusCode::NOT_FOUND);
    let voucher_gone: Option<i64> = sqlx::query_scalar("SELECT id FROM vouchers WHERE id = $1")
        .bind(voucher_id)
        .fetch_optional(&pool)
        .await?;
    assert!(voucher_gone.is_none());
    let lines_gone: i64 = sqlx::query_scalar("SELECT count(*) FROM voucher_lines WHERE voucher_id = $1")
        .bind(voucher_id)
        .fetch_one(&pool)
        .await?;
    assert_eq!(lines_gone, 0);

    Ok(())
}

#[sqlx::test(migrations = "./migrations")]
async fn amount_and_channel_validated(pool: PgPool) -> sqlx::Result<()> {
    let fx = seed(&pool).await;
    let router = app(AppState { pool: pool.clone() });

    let bad_amount = post(
        &router,
        &fx.token,
        "/api/v1/deposit-slips",
        json!({
            "fiscalYearId": fx.fiscal_year_id, "slipDate": "2018-04-01", "amount": 0,
            "payerAccountId": fx.payer_account_id, "bankAccountId": fx.bank_account_id, "channel": "pos_terminal",
        }),
    )
    .await;
    assert_eq!(bad_amount.status(), StatusCode::BAD_REQUEST);
    assert_eq!(json_body(bad_amount).await["error"], "amount_must_be_positive");

    let bad_channel = post(
        &router,
        &fx.token,
        "/api/v1/deposit-slips",
        json!({
            "fiscalYearId": fx.fiscal_year_id, "slipDate": "2018-04-01", "amount": 1000,
            "payerAccountId": fx.payer_account_id, "bankAccountId": fx.bank_account_id, "channel": "carrier_pigeon",
        }),
    )
    .await;
    assert_eq!(bad_channel.status(), StatusCode::BAD_REQUEST);
    assert_eq!(json_body(bad_channel).await["error"], "invalid_channel");

    Ok(())
}
