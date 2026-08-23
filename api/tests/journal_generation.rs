//! Automated version of step 2.6's manual test (docs/phase-2-accounting-
//! core.md §2.6): three permanently-posted vouchers touching two Kol
//! accounts roll up into one journal voucher with correct gross debit/
//! credit lines; re-running an overlapping range does not double-count; a
//! still-draft voucher in the range is rejected.

use api::{app, AppState};
use axum::{
    body::Body,
    http::{header, Request, StatusCode},
};
use serde_json::Value;
use sqlx::PgPool;
use tower::ServiceExt;

struct Fixture {
    fiscal_year_id: i64,
    cash_id: i64,
    revenue_id: i64,
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
    // The journal generator rolls up to Kol level (03-08.md §8.1), so the Kol header rows
    // themselves must exist as real accounts, not just their Moein children.
    sqlx::query("INSERT INTO accounts (tenant_id, general_ledger_code, name) VALUES ($1, 1, 'Assets')")
        .bind(tenant_id)
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO accounts (tenant_id, general_ledger_code, name) VALUES ($1, 4, 'Revenue (Kol)')")
        .bind(tenant_id)
        .execute(pool)
        .await
        .unwrap();
    let cash_id: i64 = sqlx::query_scalar(
        "INSERT INTO accounts (tenant_id, general_ledger_code, subsidiary_code, name) \
         VALUES ($1, 1, 11, 'Cash') RETURNING id",
    )
    .bind(tenant_id)
    .fetch_one(pool)
    .await
    .unwrap();
    let revenue_id: i64 = sqlx::query_scalar(
        "INSERT INTO accounts (tenant_id, general_ledger_code, subsidiary_code, name) \
         VALUES ($1, 4, 41, 'Revenue') RETURNING id",
    )
    .bind(tenant_id)
    .fetch_one(pool)
    .await
    .unwrap();
    Fixture { fiscal_year_id, cash_id, revenue_id, token }
}

fn cookie(token: &str) -> String {
    format!("arzi_session={token}")
}

async fn json_body(response: axum::response::Response) -> Value {
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

/// Creates a balanced 2-line voucher (debit cash / credit revenue) and
/// transitions it through to the given status.
async fn create_posted_voucher(
    router: &axum::Router,
    token: &str,
    fiscal_year_id: i64,
    cash_id: i64,
    revenue_id: i64,
    date: &str,
    amount: i64,
    to_status: &str,
) -> i64 {
    let create = router
        .clone()
        .oneshot(
            Request::post("/api/v1/vouchers")
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::COOKIE, cookie(token))
                .body(Body::from(
                    serde_json::json!({ "fiscalYearId": fiscal_year_id, "voucherDate": date, "description": "src" })
                        .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    let id = json_body(create).await["id"].as_i64().unwrap();

    for (account, debit, credit) in [(cash_id, amount, 0), (revenue_id, 0, amount)] {
        router
            .clone()
            .oneshot(
                Request::post(format!("/api/v1/vouchers/{id}/lines"))
                    .header(header::CONTENT_TYPE, "application/json")
                    .header(header::COOKIE, cookie(token))
                    .body(Body::from(
                        serde_json::json!({ "accountId": account, "debit": debit, "credit": credit, "description": "l" })
                            .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
    }

    for status in ["confirmed", "posted"] {
        if to_status == "draft" {
            break; // leave it as the freshly-created draft
        }
        router
            .clone()
            .oneshot(
                Request::post(format!("/api/v1/vouchers/{id}/transition"))
                    .header(header::CONTENT_TYPE, "application/json")
                    .header(header::COOKIE, cookie(token))
                    .body(Body::from(serde_json::json!({ "to": status }).to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        if status == to_status {
            break;
        }
    }

    id
}

#[sqlx::test(migrations = "./migrations")]
async fn rolls_up_posted_vouchers_and_prevents_double_counting(pool: PgPool) -> sqlx::Result<()> {
    let fx = seed(&pool).await;
    let router = app(AppState { pool: pool.clone() });

    let _v1 = create_posted_voucher(&router, &fx.token, fx.fiscal_year_id, fx.cash_id, fx.revenue_id, "2024-04-01", 100, "posted").await;
    let _v2 = create_posted_voucher(&router, &fx.token, fx.fiscal_year_id, fx.cash_id, fx.revenue_id, "2024-04-02", 200, "posted").await;
    let _v3 = create_posted_voucher(&router, &fx.token, fx.fiscal_year_id, fx.cash_id, fx.revenue_id, "2024-04-03", 50, "posted").await;

    // 2. Generate over the date range -> one journal voucher with gross debit/credit per Kol.
    let generate = router
        .clone()
        .oneshot(
            Request::post("/api/v1/vouchers/generate-journal")
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::COOKIE, cookie(&fx.token))
                .body(Body::from(
                    serde_json::json!({
                        "fiscalYearId": fx.fiscal_year_id,
                        "fromDate": "2024-04-01",
                        "toDate": "2024-04-03",
                        "voucherDate": "2024-04-04",
                        "description": "April journal roll-up"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(generate.status(), StatusCode::CREATED);
    let journal_id = json_body(generate).await["id"].as_i64().unwrap();

    let detail = router
        .clone()
        .oneshot(
            Request::get(format!("/api/v1/vouchers/{journal_id}"))
                .header(header::COOKIE, cookie(&fx.token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let body = json_body(detail).await;
    assert_eq!(body["kind"], "daybook");
    assert_eq!(body["totalDebit"], 350); // gross: 100+200+50, one Kol
    assert_eq!(body["totalCredit"], 350);
    assert_eq!(body["lineCount"], 2); // one Kol on each side -> 2 lines total

    // 3. Re-run over an overlapping range -> demonstrably does not double-count
    // (every source voucher is already journalised, so nothing new is found).
    let rerun = router
        .clone()
        .oneshot(
            Request::post("/api/v1/vouchers/generate-journal")
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::COOKIE, cookie(&fx.token))
                .body(Body::from(
                    serde_json::json!({
                        "fiscalYearId": fx.fiscal_year_id,
                        "fromDate": "2024-04-01",
                        "toDate": "2024-04-03",
                        "voucherDate": "2024-04-05",
                        "description": "Re-run attempt"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(rerun.status(), StatusCode::BAD_REQUEST);
    let body = json_body(rerun).await;
    assert_eq!(body["error"], "no_unjournalised_vouchers_in_range");

    Ok(())
}

#[sqlx::test(migrations = "./migrations")]
async fn a_still_draft_voucher_in_range_is_rejected(pool: PgPool) -> sqlx::Result<()> {
    let fx = seed(&pool).await;
    let router = app(AppState { pool: pool.clone() });

    let _posted = create_posted_voucher(&router, &fx.token, fx.fiscal_year_id, fx.cash_id, fx.revenue_id, "2024-04-01", 100, "posted").await;
    let _draft = create_posted_voucher(&router, &fx.token, fx.fiscal_year_id, fx.cash_id, fx.revenue_id, "2024-04-02", 100, "draft").await;

    let generate = router
        .oneshot(
            Request::post("/api/v1/vouchers/generate-journal")
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::COOKIE, cookie(&fx.token))
                .body(Body::from(
                    serde_json::json!({
                        "fiscalYearId": fx.fiscal_year_id,
                        "fromDate": "2024-04-01",
                        "toDate": "2024-04-02",
                        "voucherDate": "2024-04-03",
                        "description": "Should fail: draft in range"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(generate.status(), StatusCode::BAD_REQUEST);
    let body = json_body(generate).await;
    assert_eq!(body["error"], "vouchers_not_all_posted");

    Ok(())
}
