//! Automated version of step 6.6's manual test (docs/phase-6-reporting.md
//! §6.6): B17 (drafts excluded from the tax-authority export), correct
//! column labelling, comma/whitespace handling, and a clean re-importable
//! `.xlsx`.

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
    token: String,
    cash_account_id: i64,
    sales_account_id: i64,
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
         VALUES ($1, 1406, '2027-03-21', '2028-03-20') RETURNING id",
    )
    .bind(tenant_id)
    .fetch_one(pool)
    .await
    .unwrap();
    sqlx::query("INSERT INTO accounts (tenant_id, general_ledger_code, name) VALUES ($1, 10, 'Assets')")
        .bind(tenant_id)
        .execute(pool)
        .await
        .unwrap();
    let cash_account_id: i64 = sqlx::query_scalar(
        "INSERT INTO accounts (tenant_id, general_ledger_code, subsidiary_code, name) \
         VALUES ($1, 10, 1, 'Cash') RETURNING id",
    )
    .bind(tenant_id)
    .fetch_one(pool)
    .await
    .unwrap();
    sqlx::query("INSERT INTO accounts (tenant_id, general_ledger_code, name) VALUES ($1, 40, 'Revenue')")
        .bind(tenant_id)
        .execute(pool)
        .await
        .unwrap();
    let sales_account_id: i64 = sqlx::query_scalar(
        "INSERT INTO accounts (tenant_id, general_ledger_code, subsidiary_code, name) \
         VALUES ($1, 40, 1, 'Sales') RETURNING id",
    )
    .bind(tenant_id)
    .fetch_one(pool)
    .await
    .unwrap();
    Fixture { fiscal_year_id, token, cash_account_id, sales_account_id }
}

fn cookie(token: &str) -> String {
    format!("arzi_session={token}")
}

async fn json_body(response: axum::response::Response) -> Value {
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

/// Creates a voucher on `date` with a debit/credit pair for `amount`, with
/// `description` as the line narration, optionally posted.
async fn make_voucher(router: &axum::Router, token: &str, fx: &Fixture, date: &str, amount: i64, description: &str, post: bool) {
    let create = router
        .clone()
        .oneshot(
            Request::post("/api/v1/vouchers")
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::COOKIE, cookie(token))
                .body(Body::from(json!({ "fiscalYearId": fx.fiscal_year_id, "voucherDate": date, "description": "x" }).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(create.status(), StatusCode::CREATED);
    let voucher_id = json_body(create).await["id"].as_i64().unwrap();

    let add_line = |acc: i64, debit: i64, credit: i64, desc: &str| {
        let router = router.clone();
        let token = token.to_string();
        let desc = desc.to_string();
        async move {
            router
                .oneshot(
                    Request::post(format!("/api/v1/vouchers/{voucher_id}/lines"))
                        .header(header::CONTENT_TYPE, "application/json")
                        .header(header::COOKIE, cookie(&token))
                        .body(Body::from(json!({ "accountId": acc, "debit": debit, "credit": credit, "description": desc }).to_string()))
                        .unwrap(),
                )
                .await
                .unwrap()
        }
    };
    assert_eq!(add_line(fx.cash_account_id, amount, 0, description).await.status(), StatusCode::CREATED);
    assert_eq!(add_line(fx.sales_account_id, 0, amount, "contra").await.status(), StatusCode::CREATED);

    if post {
        let transition = |to: &'static str| {
            let router = router.clone();
            let token = token.to_string();
            async move {
                router
                    .oneshot(
                        Request::post(format!("/api/v1/vouchers/{voucher_id}/transition"))
                            .header(header::CONTENT_TYPE, "application/json")
                            .header(header::COOKIE, cookie(&token))
                            .body(Body::from(json!({ "to": to }).to_string()))
                            .unwrap(),
                    )
                    .await
                    .unwrap()
            }
        };
        assert_eq!(transition("confirmed").await.status(), StatusCode::NO_CONTENT);
        assert_eq!(transition("posted").await.status(), StatusCode::NO_CONTENT);
    }
}

/// Manual test #1/#2/#3 in one flow: drafts excluded (B17), the voucher-
/// number column is correctly headed, and a comma/whitespace-laden
/// narration survives a real CSV round trip intact.
#[sqlx::test(migrations = "./migrations")]
async fn tax_export_excludes_drafts_and_labels_and_quotes_correctly(pool: PgPool) -> sqlx::Result<()> {
    let fx = seed(&pool).await;
    let router = app(AppState { pool: pool.clone() });

    make_voucher(&router, &fx.token, &fx, "2027-04-01", 1_000_000, "فروش، شامل   چند   فاصله", true).await;
    // Left as a draft -- must be excluded entirely (B17, direct test #1).
    make_voucher(&router, &fx.token, &fx, "2027-04-05", 9_999_999, "draft only", false).await;

    let resp = router
        .oneshot(
            Request::get(format!(
                "/api/v1/reports/tax-authority-export?fiscalYearId={}&fromDate=2000-01-01&toDate=2028-01-01&format=csv",
                fx.fiscal_year_id
            ))
            .header(header::COOKIE, cookie(&fx.token))
            .body(Body::empty())
            .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(resp.headers().get(header::CONTENT_TYPE).unwrap(), "text/csv; charset=utf-8");
    let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let text = String::from_utf8(bytes.to_vec()).unwrap();

    // Manual test #2: the voucher-number column is headed correctly, not "ردیف".
    assert!(text.starts_with("شماره سند,تاریخ,کل,نام کل,معین,نام معین,شرح,مبلغ بدهکار,مبلغ بستانکار"));

    // Manual test #1 (B17): the draft's distinctive 9,999,999 never appears.
    assert!(!text.contains("9999999"));

    // Manual test #3: real CSV round trip -- the comma-and-multi-space
    // narration survives as one field, whitespace collapsed to single spaces.
    let mut reader = csv::Reader::from_reader(text.as_bytes());
    let mut found = false;
    for record in reader.records() {
        let record = record.unwrap();
        if record[6].contains('،') {
            assert_eq!(&record[6], "فروش، شامل چند فاصله");
            assert_eq!(&record[7], "1000000");
            found = true;
        }
    }
    assert!(found, "the posted line's narration must appear intact as one CSV field");

    Ok(())
}

/// Manual test #4: the exported `.xlsx` is a real, clean workbook.
#[sqlx::test(migrations = "./migrations")]
async fn tax_export_xlsx_is_a_real_clean_workbook(pool: PgPool) -> sqlx::Result<()> {
    let fx = seed(&pool).await;
    let router = app(AppState { pool: pool.clone() });
    make_voucher(&router, &fx.token, &fx, "2027-04-01", 500_000, "test", true).await;

    let resp = router
        .oneshot(
            Request::get(format!(
                "/api/v1/reports/tax-authority-export?fiscalYearId={}&fromDate=2000-01-01&toDate=2028-01-01&format=xlsx",
                fx.fiscal_year_id
            ))
            .header(header::COOKIE, cookie(&fx.token))
            .body(Body::empty())
            .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(
        resp.headers().get(header::CONTENT_TYPE).unwrap(),
        "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"
    );
    let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    assert!(bytes.starts_with(b"PK"), "a real xlsx (zip archive)");

    Ok(())
}
