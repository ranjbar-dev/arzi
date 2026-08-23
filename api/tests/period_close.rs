//! Automated version of step 2.7's manual test (docs/phase-2-accounting-
//! core.md §2.7): non-admin rejected on both operations; carry-forward
//! rejected until close-books has run and been posted (A7); close-books
//! zeroes income-statement accounts into a destination, balanced by
//! construction; carry-forward then succeeds and re-establishes the
//! remaining (balance-sheet) balances in the next fiscal year; a destination
//! that is itself one of the ticked source accounts is rejected by id, not
//! by the legacy's dash-dependent string check.

use api::{app, AppState};
use axum::{
    body::Body,
    http::{header, Request, StatusCode},
};
use serde_json::{json, Value};
use sqlx::PgPool;
use tower::ServiceExt;

struct Fixture {
    outgoing_fy: i64,
    incoming_fy: i64,
    cash_id: i64,
    revenue_kol_id: i64,
    revenue_id: i64,
    expense_kol_id: i64,
    expense_id: i64,
    summary_id: i64,
    closing_contra_id: i64,
    opening_contra_id: i64,
    admin_token: String,
    plain_token: String,
}

async fn seed(pool: &PgPool) -> Fixture {
    let tenant_id: i64 = sqlx::query_scalar("INSERT INTO tenants (slug, name) VALUES ('acme', 'Acme') RETURNING id")
        .fetch_one(pool)
        .await
        .unwrap();

    async fn make_user(pool: &PgPool, tenant_id: i64, username: &str, superuser: bool) -> String {
        let user_id: i64 = sqlx::query_scalar(
            "INSERT INTO users (tenant_id, username, password_hash, is_superuser) \
             VALUES ($1, $2, 'x', $3) RETURNING id",
        )
        .bind(tenant_id)
        .bind(username)
        .bind(superuser)
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
        token
    }

    let admin_token = make_user(pool, tenant_id, "root", true).await;
    let plain_token = make_user(pool, tenant_id, "clerk", false).await;

    let outgoing_fy: i64 = sqlx::query_scalar(
        "INSERT INTO fiscal_years (tenant_id, year, start_date, end_date) \
         VALUES ($1, 1403, '2024-03-20', '2025-03-20') RETURNING id",
    )
    .bind(tenant_id)
    .fetch_one(pool)
    .await
    .unwrap();
    let incoming_fy: i64 = sqlx::query_scalar(
        "INSERT INTO fiscal_years (tenant_id, year, start_date, end_date) \
         VALUES ($1, 1404, '2025-03-21', '2026-03-20') RETURNING id",
    )
    .bind(tenant_id)
    .fetch_one(pool)
    .await
    .unwrap();

    async fn leaf(pool: &PgPool, tenant_id: i64, kol: i32, moein: i32, name: &str) -> i64 {
        sqlx::query_scalar(
            "INSERT INTO accounts (tenant_id, general_ledger_code, subsidiary_code, name) \
             VALUES ($1, $2, $3, $4) RETURNING id",
        )
        .bind(tenant_id)
        .bind(kol)
        .bind(moein)
        .bind(name)
        .fetch_one(pool)
        .await
        .unwrap()
    }
    async fn kol(pool: &PgPool, tenant_id: i64, code: i32, name: &str) -> i64 {
        sqlx::query_scalar(
            "INSERT INTO accounts (tenant_id, general_ledger_code, name) VALUES ($1, $2, $3) RETURNING id",
        )
        .bind(tenant_id)
        .bind(code)
        .bind(name)
        .fetch_one(pool)
        .await
        .unwrap()
    }

    let cash_id = leaf(pool, tenant_id, 1, 11, "Cash").await;
    let revenue_kol_id = kol(pool, tenant_id, 4, "Revenue (Kol)").await;
    let revenue_id = leaf(pool, tenant_id, 4, 41, "Revenue").await;
    let expense_kol_id = kol(pool, tenant_id, 5, "Expense (Kol)").await;
    let expense_id = leaf(pool, tenant_id, 5, 51, "Expense").await;
    let summary_id = leaf(pool, tenant_id, 9, 91, "P&L Summary").await;
    let closing_contra_id = leaf(pool, tenant_id, 2, 21, "Closing suspense").await;
    let opening_contra_id = leaf(pool, tenant_id, 3, 31, "Opening suspense").await;

    Fixture {
        outgoing_fy,
        incoming_fy,
        cash_id,
        revenue_kol_id,
        revenue_id,
        expense_kol_id,
        expense_id,
        summary_id,
        closing_contra_id,
        opening_contra_id,
        admin_token,
        plain_token,
    }
}

fn cookie(token: &str) -> String {
    format!("arzi_session={token}")
}

async fn json_body(response: axum::response::Response) -> Value {
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
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

/// Creates a balanced two-line voucher and drives it to `posted`.
async fn post_voucher(
    router: &axum::Router,
    token: &str,
    fiscal_year_id: i64,
    date: &str,
    debit_account: i64,
    credit_account: i64,
    amount: i64,
) -> i64 {
    let create = req(
        router,
        "POST",
        "/api/v1/vouchers",
        token,
        json!({ "fiscalYearId": fiscal_year_id, "voucherDate": date, "description": "seed" }),
    )
    .await;
    let id = json_body(create).await["id"].as_i64().unwrap();
    for (account, debit, credit) in [(debit_account, amount, 0), (credit_account, 0, amount)] {
        req(
            router,
            "POST",
            &format!("/api/v1/vouchers/{id}/lines"),
            token,
            json!({ "accountId": account, "debit": debit, "credit": credit, "description": "l" }),
        )
        .await;
    }
    for to in ["confirmed", "posted"] {
        req(router, "POST", &format!("/api/v1/vouchers/{id}/transition"), token, json!({ "to": to })).await;
    }
    id
}

#[sqlx::test(migrations = "./migrations")]
async fn non_admin_rejected_and_carry_forward_blocked_until_close(pool: PgPool) -> sqlx::Result<()> {
    let fx = seed(&pool).await;
    let router = app(AppState { pool: pool.clone() });

    // 1. non-admin -> rejected on both.
    let close_as_plain = req(
        &router,
        "POST",
        &format!("/api/v1/fiscal-years/{}/close-books", fx.outgoing_fy),
        &fx.plain_token,
        json!({
            "sourceKolAccountIds": [fx.revenue_kol_id],
            "destinationAccountId": fx.summary_id,
            "voucherDate": "2025-03-19",
            "description": "close"
        }),
    )
    .await;
    assert_eq!(close_as_plain.status(), StatusCode::FORBIDDEN);

    let carry_as_plain = req(
        &router,
        "POST",
        &format!("/api/v1/fiscal-years/{}/carry-forward", fx.outgoing_fy),
        &fx.plain_token,
        json!({
            "closingDate": "2025-03-20", "openingDate": "2025-03-21",
            "closingDescription": "c", "openingDescription": "o",
            "closingContraAccountId": fx.closing_contra_id, "openingContraAccountId": fx.opening_contra_id
        }),
    )
    .await;
    assert_eq!(carry_as_plain.status(), StatusCode::FORBIDDEN);

    // 2. admin, carry-forward before close-books has ever run -> rejected, hard error.
    let carry_before_close = req(
        &router,
        "POST",
        &format!("/api/v1/fiscal-years/{}/carry-forward", fx.outgoing_fy),
        &fx.admin_token,
        json!({
            "closingDate": "2025-03-20", "openingDate": "2025-03-21",
            "closingDescription": "c", "openingDescription": "o",
            "closingContraAccountId": fx.closing_contra_id, "openingContraAccountId": fx.opening_contra_id
        }),
    )
    .await;
    assert_eq!(carry_before_close.status(), StatusCode::BAD_REQUEST);
    assert_eq!(json_body(carry_before_close).await["error"], "books_not_closed");

    Ok(())
}

#[sqlx::test(migrations = "./migrations")]
async fn destination_in_source_rejected_by_id_not_string_shape(pool: PgPool) -> sqlx::Result<()> {
    let fx = seed(&pool).await;
    let router = app(AppState { pool: pool.clone() });

    post_voucher(&router, &fx.admin_token, fx.outgoing_fy, "2024-04-01", fx.cash_id, fx.revenue_id, 1000).await;

    // destination = the revenue LEAF itself, under the ticked revenue Kol. This is exactly the
    // shape the legacy's sentinel-padded string test would fail to catch when a code has no dash
    // (03-09-a.md §9.2 validation 7) -- here it's an id/code comparison, so it's caught regardless.
    let resp = req(
        &router,
        "POST",
        &format!("/api/v1/fiscal-years/{}/close-books", fx.outgoing_fy),
        &fx.admin_token,
        json!({
            "sourceKolAccountIds": [fx.revenue_kol_id],
            "destinationAccountId": fx.revenue_id,
            "voucherDate": "2025-03-19",
            "description": "close"
        }),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    assert_eq!(json_body(resp).await["error"], "destination_in_source");

    Ok(())
}

#[sqlx::test(migrations = "./migrations")]
async fn close_then_carry_forward_succeeds(pool: PgPool) -> sqlx::Result<()> {
    let fx = seed(&pool).await;
    let router = app(AppState { pool: pool.clone() });

    // Revenue 1000 credit, Expense 300 debit, leaving Cash at a net debit of 700.
    post_voucher(&router, &fx.admin_token, fx.outgoing_fy, "2024-04-01", fx.cash_id, fx.revenue_id, 1000).await;
    post_voucher(&router, &fx.admin_token, fx.outgoing_fy, "2024-04-02", fx.expense_id, fx.cash_id, 300).await;

    // 3. close-books: zero Revenue + Expense into the summary destination.
    let close = req(
        &router,
        "POST",
        &format!("/api/v1/fiscal-years/{}/close-books", fx.outgoing_fy),
        &fx.admin_token,
        json!({
            "sourceKolAccountIds": [fx.revenue_kol_id, fx.expense_kol_id],
            "destinationAccountId": fx.summary_id,
            "voucherDate": "2025-03-19",
            "description": "close the books"
        }),
    )
    .await;
    assert_eq!(close.status(), StatusCode::CREATED);
    let close_voucher_id = json_body(close).await["id"].as_i64().unwrap();

    let close_detail = json_body(req(&router, "GET", &format!("/api/v1/vouchers/{close_voucher_id}"), &fx.admin_token, Value::Null).await).await;
    assert_eq!(close_detail["totalDebit"], 1300); // balanced by construction
    assert_eq!(close_detail["totalCredit"], 1300);
    assert_eq!(close_detail["lineCount"], 4); // 2 leaves x 2 lines each

    // carry-forward still blocked: the close-books voucher exists but isn't posted yet.
    let too_early = req(
        &router,
        "POST",
        &format!("/api/v1/fiscal-years/{}/carry-forward", fx.outgoing_fy),
        &fx.admin_token,
        json!({
            "closingDate": "2025-03-20", "openingDate": "2025-03-21",
            "closingDescription": "c", "openingDescription": "o",
            "closingContraAccountId": fx.closing_contra_id, "openingContraAccountId": fx.opening_contra_id
        }),
    )
    .await;
    assert_eq!(too_early.status(), StatusCode::BAD_REQUEST);
    assert_eq!(json_body(too_early).await["error"], "books_not_closed");

    // 4. post the closing voucher.
    for to in ["confirmed", "posted"] {
        let t = req(&router, "POST", &format!("/api/v1/vouchers/{close_voucher_id}/transition"), &fx.admin_token, json!({ "to": to })).await;
        assert_eq!(t.status(), StatusCode::NO_CONTENT);
    }

    // 5. carry-forward now succeeds.
    let carry = req(
        &router,
        "POST",
        &format!("/api/v1/fiscal-years/{}/carry-forward", fx.outgoing_fy),
        &fx.admin_token,
        json!({
            "closingDate": "2025-03-20", "openingDate": "2025-03-21",
            "closingDescription": "closing entry", "openingDescription": "opening entry",
            "closingContraAccountId": fx.closing_contra_id, "openingContraAccountId": fx.opening_contra_id
        }),
    )
    .await;
    assert_eq!(carry.status(), StatusCode::CREATED);
    let body = json_body(carry).await;
    let opening_voucher_id = body["openingVoucherId"].as_i64().unwrap();

    // Opening balances in the new fiscal year match the prior year's remaining
    // (post-close, balance-sheet) balances: Cash net debit 700, Summary net credit 700.
    let opening_detail = json_body(req(&router, "GET", &format!("/api/v1/vouchers/{opening_voucher_id}"), &fx.admin_token, Value::Null).await).await;
    assert_eq!(opening_detail["totalDebit"], 1400); // (700 Cash + 700 Summary), balanced
    assert_eq!(opening_detail["totalCredit"], 1400);
    let lines = opening_detail["lines"].as_array().unwrap();
    let cash_line = lines.iter().find(|l| l["accountId"] == fx.cash_id).unwrap();
    assert_eq!(cash_line["debitAmount"], 700);
    assert_eq!(cash_line["creditAmount"], 0);
    let summary_line = lines.iter().find(|l| l["accountId"] == fx.summary_id).unwrap();
    assert_eq!(summary_line["creditAmount"], 700);
    assert_eq!(summary_line["debitAmount"], 0);

    // A second carry-forward attempt is rejected -- but at check #2, not #15: the first run's
    // own closing voucher is itself a fresh draft in the outgoing year now (03-09-b.md's "nothing
    // is locked afterwards"), so "all vouchers finalised" fails before the balance-emptiness check
    // is ever reached.
    let rerun = req(
        &router,
        "POST",
        &format!("/api/v1/fiscal-years/{}/carry-forward", fx.outgoing_fy),
        &fx.admin_token,
        json!({
            "closingDate": "2025-03-20", "openingDate": "2025-03-21",
            "closingDescription": "c2", "openingDescription": "o2",
            "closingContraAccountId": fx.closing_contra_id, "openingContraAccountId": fx.opening_contra_id
        }),
    )
    .await;
    assert_eq!(rerun.status(), StatusCode::BAD_REQUEST);
    assert_eq!(json_body(rerun).await["error"], "vouchers_not_all_posted");

    // Post that new closing voucher too -- now the driving query genuinely finds nothing left
    // (self-limiting, #15): both Cash and Summary already net to zero in the outgoing year.
    for to in ["confirmed", "posted"] {
        req(&router, "POST", &format!("/api/v1/vouchers/{}/transition", body["closingVoucherId"]), &fx.admin_token, json!({ "to": to })).await;
    }
    let rerun2 = req(
        &router,
        "POST",
        &format!("/api/v1/fiscal-years/{}/carry-forward", fx.outgoing_fy),
        &fx.admin_token,
        json!({
            "closingDate": "2025-03-20", "openingDate": "2025-03-21",
            "closingDescription": "c3", "openingDescription": "o3",
            "closingContraAccountId": fx.closing_contra_id, "openingContraAccountId": fx.opening_contra_id
        }),
    )
    .await;
    assert_eq!(rerun2.status(), StatusCode::BAD_REQUEST);
    assert_eq!(json_body(rerun2).await["error"], "already_carried_forward");

    Ok(())
}
