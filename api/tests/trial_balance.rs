//! Automated version of step 6.1's manual test (docs/phase-6-reporting.md
//! §6.1): drafts excluded (A8), the balance proof, fiscal-year scoping, and
//! the grand total matching the Kol-level total regardless of chosen detail
//! level.

use api::{app, AppState};
use axum::{
    body::Body,
    http::{header, Request, StatusCode},
};
use serde_json::{json, Value};
use sqlx::PgPool;
use tower::ServiceExt;

struct Fixture {
    tenant_id: i64,
    fiscal_year_id: i64,
    token: String,
}

async fn seed(pool: &PgPool) -> Fixture {
    let tenant_id: i64 =
        sqlx::query_scalar("INSERT INTO tenants (slug, name) VALUES ('acme', 'Acme') RETURNING id")
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
         VALUES ($1, 1397, '2018-03-21', '2019-03-20') RETURNING id",
    )
    .bind(tenant_id)
    .fetch_one(pool)
    .await
    .unwrap();
    Fixture {
        tenant_id,
        fiscal_year_id,
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

async fn seed_account(
    pool: &PgPool,
    tenant_id: i64,
    gl: i32,
    sub: i32,
    a1: i32,
    a2: i32,
    name: &str,
) -> i64 {
    sqlx::query_scalar(
        "INSERT INTO accounts (tenant_id, general_ledger_code, subsidiary_code, analytic1_code, \
         analytic2_code, name) VALUES ($1, $2, $3, $4, $5, $6) RETURNING id",
    )
    .bind(tenant_id)
    .bind(gl)
    .bind(sub)
    .bind(a1)
    .bind(a2)
    .bind(name)
    .fetch_one(pool)
    .await
    .unwrap()
}

/// Creates a voucher on `date`, adds one line to each of `account_id`
/// (debit) and `contra_id` (credit) for `amount`, then optionally drives it
/// through confirm/post. Returns the voucher id.
async fn make_voucher(
    router: &axum::Router,
    token: &str,
    fiscal_year_id: i64,
    date: &str,
    account_id: i64,
    contra_id: i64,
    amount: i64,
    post: bool,
) -> i64 {
    let create = router
        .clone()
        .oneshot(
            Request::post("/api/v1/vouchers")
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::COOKIE, cookie(token))
                .body(Body::from(
                    json!({ "fiscalYearId": fiscal_year_id, "voucherDate": date, "description": "trial balance fixture" })
                        .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(create.status(), StatusCode::CREATED);
    let voucher_id = json_body(create).await["id"].as_i64().unwrap();

    let add_line = |acc: i64, debit: i64, credit: i64| {
        let router = router.clone();
        let token = token.to_string();
        async move {
            router
                .oneshot(
                    Request::post(format!("/api/v1/vouchers/{voucher_id}/lines"))
                        .header(header::CONTENT_TYPE, "application/json")
                        .header(header::COOKIE, cookie(&token))
                        .body(Body::from(
                            json!({ "accountId": acc, "debit": debit, "credit": credit, "description": "line" })
                                .to_string(),
                        ))
                        .unwrap(),
                )
                .await
                .unwrap()
        }
    };
    assert_eq!(
        add_line(account_id, amount, 0).await.status(),
        StatusCode::CREATED
    );
    assert_eq!(
        add_line(contra_id, 0, amount).await.status(),
        StatusCode::CREATED
    );

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
        assert_eq!(
            transition("confirmed").await.status(),
            StatusCode::NO_CONTENT
        );
        assert_eq!(transition("posted").await.status(), StatusCode::NO_CONTENT);
    }
    voucher_id
}

/// Manual test #1/#2/#4: two Kol accounts, several posted vouchers, one
/// draft left unposted -> the draft is excluded (A8), the balance proof is
/// true, and the grand total matches the Kol-level total at every requested
/// detail level.
#[sqlx::test(migrations = "./migrations")]
async fn four_column_excludes_drafts_and_proves_balance(pool: PgPool) -> sqlx::Result<()> {
    let fx = seed(&pool).await;
    let router = app(AppState { pool: pool.clone() });

    seed_account(&pool, fx.tenant_id, 10, 0, 0, 0, "Assets").await;
    let cash = seed_account(&pool, fx.tenant_id, 10, 1, 0, 0, "Cash").await;
    seed_account(&pool, fx.tenant_id, 40, 0, 0, 0, "Revenue").await;
    let sales = seed_account(&pool, fx.tenant_id, 40, 1, 0, 0, "Sales").await;

    // Two posted vouchers moving 1,000,000 and 500,000 through Cash/Sales.
    make_voucher(
        &router,
        &fx.token,
        fx.fiscal_year_id,
        "2018-04-01",
        cash,
        sales,
        1_000_000,
        true,
    )
    .await;
    make_voucher(
        &router,
        &fx.token,
        fx.fiscal_year_id,
        "2018-04-05",
        cash,
        sales,
        500_000,
        true,
    )
    .await;
    // A third, left as a draft -- must be excluded entirely (A8/direct test #1).
    make_voucher(
        &router,
        &fx.token,
        fx.fiscal_year_id,
        "2018-04-10",
        cash,
        sales,
        9_999_999,
        false,
    )
    .await;

    let resp = router
        .clone()
        .oneshot(
            Request::get(format!(
                "/api/v1/reports/trial-balance-4-column?fiscalYearId={}&asOfDate=2018-12-31&level=moein",
                fx.fiscal_year_id
            ))
            .header(header::COOKIE, cookie(&fx.token))
            .body(Body::empty())
            .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = json_body(resp).await;

    // Draft's 9,999,999 must appear nowhere.
    let rows = body["rows"].as_array().unwrap();
    for row in rows {
        assert_ne!(row["cumulativeDebit"], 9_999_999);
        assert_ne!(row["cumulativeCredit"], 9_999_999);
    }

    // Kol(Assets) + Moein(Cash) rows, both showing the posted-only 1,500,000.
    let kol_assets = rows
        .iter()
        .find(|r| r["level"] == 1 && r["generalLedgerCode"] == 10)
        .expect("Kol Assets row present");
    assert_eq!(kol_assets["cumulativeDebit"], 1_500_000);
    assert_eq!(kol_assets["balanceDebit"], 1_500_000);
    assert_eq!(kol_assets["balanceCredit"], 0);

    let moein_cash = rows
        .iter()
        .find(|r| r["level"] == 2 && r["generalLedgerCode"] == 10 && r["subsidiaryCode"] == 1)
        .expect("Moein Cash row present");
    assert_eq!(moein_cash["cumulativeDebit"], 1_500_000);

    // Manual test #2: the balance proof is explicitly present and true.
    assert_eq!(body["balanceProof"]["totalDebit"], 1_500_000);
    assert_eq!(body["balanceProof"]["totalCredit"], 1_500_000);
    assert_eq!(body["balanceProof"]["balanced"], true);

    // Manual test #4: the grand total is the Kol-level total, not a naive
    // sum over both interleaved levels (which would double it to 3,000,000).
    assert_eq!(body["grandTotal"]["cumulativeDebit"], 1_500_000);
    assert_eq!(body["grandTotal"]["cumulativeCredit"], 1_500_000);

    Ok(())
}

/// Manual test #3: a second fiscal year with its own posting must not leak
/// into the first year's report.
#[sqlx::test(migrations = "./migrations")]
async fn fiscal_year_selector_actually_scopes_the_query(pool: PgPool) -> sqlx::Result<()> {
    let fx = seed(&pool).await;
    let router = app(AppState { pool: pool.clone() });

    seed_account(&pool, fx.tenant_id, 10, 0, 0, 0, "Assets").await;
    let cash = seed_account(&pool, fx.tenant_id, 10, 1, 0, 0, "Cash").await;
    seed_account(&pool, fx.tenant_id, 40, 0, 0, 0, "Revenue").await;
    let sales = seed_account(&pool, fx.tenant_id, 40, 1, 0, 0, "Sales").await;
    make_voucher(
        &router,
        &fx.token,
        fx.fiscal_year_id,
        "2018-04-01",
        cash,
        sales,
        1_000_000,
        true,
    )
    .await;

    let other_fy: i64 = sqlx::query_scalar(
        "INSERT INTO fiscal_years (tenant_id, year, start_date, end_date) \
         VALUES ($1, 1398, '2019-03-21', '2020-03-20') RETURNING id",
    )
    .bind(fx.tenant_id)
    .fetch_one(&pool)
    .await?;
    make_voucher(
        &router,
        &fx.token,
        other_fy,
        "2019-04-01",
        cash,
        sales,
        7_000_000,
        true,
    )
    .await;

    let resp = router
        .oneshot(
            Request::get(format!(
                "/api/v1/reports/trial-balance-4-column?fiscalYearId={}&asOfDate=2018-12-31&level=kol",
                fx.fiscal_year_id
            ))
            .header(header::COOKIE, cookie(&fx.token))
            .body(Body::empty())
            .unwrap(),
        )
        .await
        .unwrap();
    let body = json_body(resp).await;
    assert_eq!(
        body["balanceProof"]["totalDebit"], 1_000_000,
        "must not include the other fiscal year's 7,000,000"
    );

    Ok(())
}

/// Manual test for the 6-column report's opening/period/closing split, plus
/// its single-level (not cumulative) rendering and its own balance proof.
#[sqlx::test(migrations = "./migrations")]
async fn six_column_splits_opening_period_and_closing(pool: PgPool) -> sqlx::Result<()> {
    let fx = seed(&pool).await;
    let router = app(AppState { pool: pool.clone() });

    let cash = seed_account(&pool, fx.tenant_id, 10, 1, 0, 0, "Cash").await;
    let sales = seed_account(&pool, fx.tenant_id, 40, 1, 0, 0, "Sales").await;
    // Before the period: 400,000. During the period: 100,000.
    make_voucher(
        &router,
        &fx.token,
        fx.fiscal_year_id,
        "2018-04-01",
        cash,
        sales,
        400_000,
        true,
    )
    .await;
    make_voucher(
        &router,
        &fx.token,
        fx.fiscal_year_id,
        "2018-06-15",
        cash,
        sales,
        100_000,
        true,
    )
    .await;

    let resp = router
        .oneshot(
            Request::get(format!(
                "/api/v1/reports/trial-balance-6-column?fiscalYearId={}&fromDate=2018-06-01&toDate=2018-06-30&level=moein",
                fx.fiscal_year_id
            ))
            .header(header::COOKIE, cookie(&fx.token))
            .body(Body::empty())
            .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = json_body(resp).await;

    // Single level only -- no Kol-level row mixed into the output.
    let rows = body["rows"].as_array().unwrap();
    assert!(rows.iter().all(|r| r["level"] == 2));

    let cash_row = rows
        .iter()
        .find(|r| r["generalLedgerCode"] == 10)
        .expect("cash row present");
    assert_eq!(cash_row["openingDebit"], 400_000);
    assert_eq!(cash_row["periodDebit"], 100_000);
    assert_eq!(cash_row["closingDebit"], 500_000);
    assert_eq!(cash_row["closingCredit"], 0);

    assert_eq!(body["balanceProof"]["balanced"], true);
    assert_eq!(body["balanceProof"]["openingBalanced"], true);

    Ok(())
}

/// The 04-02-b.md §2.2 validation-bug fix: an inverted range is rejected
/// outright, not silently run to an empty result.
#[sqlx::test(migrations = "./migrations")]
async fn six_column_rejects_inverted_date_range(pool: PgPool) -> sqlx::Result<()> {
    let fx = seed(&pool).await;
    let router = app(AppState { pool });

    let resp = router
        .oneshot(
            Request::get(format!(
                "/api/v1/reports/trial-balance-6-column?fiscalYearId={}&fromDate=2018-06-30&toDate=2018-06-01",
                fx.fiscal_year_id
            ))
            .header(header::COOKIE, cookie(&fx.token))
            .body(Body::empty())
            .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

    Ok(())
}
