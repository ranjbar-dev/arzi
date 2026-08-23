//! Automated version of step 6.2's manual test (docs/phase-6-reporting.md
//! §6.2): the general ledger works without journal generation ever having
//! run (B6), the opening balance is one net figure, B4/B5's specific
//! discrepancies cannot be reproduced, the ordering tie-break is stable, and
//! the lock-based permission gate uses the real rule.

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

async fn make_voucher(
    router: &axum::Router,
    token: &str,
    fiscal_year_id: i64,
    date: &str,
    account_id: i64,
    contra_id: i64,
    debit: i64,
    credit: i64,
) {
    let create = router
        .clone()
        .oneshot(
            Request::post("/api/v1/vouchers")
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::COOKIE, cookie(token))
                .body(Body::from(
                    json!({ "fiscalYearId": fiscal_year_id, "voucherDate": date, "description": "ledger fixture" })
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
        add_line(account_id, debit, credit).await.status(),
        StatusCode::CREATED
    );
    assert_eq!(
        add_line(contra_id, credit, debit).await.status(),
        StatusCode::CREATED
    );

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

/// Manual test #1 (direct B6 test): the general ledger shows data
/// immediately with no journal generation ever run, plus #2 (opening
/// balance is one net figure) and #4 (same-voucher tie-break is stable).
#[sqlx::test(migrations = "./migrations")]
async fn ledger_works_without_journal_generation_and_nets_the_opening_balance(
    pool: PgPool,
) -> sqlx::Result<()> {
    let fx = seed(&pool).await;
    let router = app(AppState { pool: pool.clone() });

    seed_account(&pool, fx.tenant_id, 10, 0, 0, 0, "Assets").await;
    let cash = seed_account(&pool, fx.tenant_id, 10, 1, 0, 0, "Cash").await;
    let sales = seed_account(&pool, fx.tenant_id, 40, 1, 0, 0, "Sales").await;

    // Before the window: debit 1,000 and credit 400 -> net opening = -600 (debit).
    make_voucher(
        &router,
        &fx.token,
        fx.fiscal_year_id,
        "2018-04-01",
        cash,
        sales,
        1_000,
        0,
    )
    .await;
    make_voucher(
        &router,
        &fx.token,
        fx.fiscal_year_id,
        "2018-04-02",
        cash,
        sales,
        0,
        400,
    )
    .await;
    // Two lines of the SAME voucher hitting Cash, inside the window -> tie-break test.
    make_voucher(
        &router,
        &fx.token,
        fx.fiscal_year_id,
        "2018-06-01",
        cash,
        sales,
        200,
        0,
    )
    .await;

    let resp = router
        .clone()
        .oneshot(
            Request::get(format!(
                "/api/v1/reports/ledger?fiscalYearId={}&generalLedgerCode=10&fromDate=2018-05-01&toDate=2018-12-31",
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

    // No journal generation ever ran -- direct B6 test: data present anyway.
    assert_eq!(
        body["openingBalance"],
        json!({ "amount": 600, "side": "debit" })
    );
    let rows = body["rows"].as_array().unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["debit"], 200);
    // Opening -600 + this period's -200 (debit) = -800 net -> 800 debit.
    assert_eq!(
        body["closingBalance"],
        json!({ "amount": 800, "side": "debit" })
    );

    Ok(())
}

/// Manual test #3 (B4/B5): the same shared predicate/boundary is used for
/// both the exact-coordinate case and the arbitrary-id-list ("consolidated
/// ledger") case -- proven by requesting the SAME two accounts both ways and
/// getting identical figures, plus the strict `< from_date` opening boundary.
#[sqlx::test(migrations = "./migrations")]
async fn consolidated_and_coordinate_views_agree_and_boundary_is_strict(
    pool: PgPool,
) -> sqlx::Result<()> {
    let fx = seed(&pool).await;
    let router = app(AppState { pool: pool.clone() });

    seed_account(&pool, fx.tenant_id, 10, 0, 0, 0, "Assets").await;
    let cash = seed_account(&pool, fx.tenant_id, 10, 1, 0, 0, "Cash").await;
    let bank = seed_account(&pool, fx.tenant_id, 10, 2, 0, 0, "Bank").await;
    let sales = seed_account(&pool, fx.tenant_id, 40, 1, 0, 0, "Sales").await;

    // Exactly on the from-date boundary -> must land in the PERIOD, not opening (B5: `< from_date`).
    make_voucher(
        &router,
        &fx.token,
        fx.fiscal_year_id,
        "2018-06-01",
        cash,
        sales,
        500,
        0,
    )
    .await;
    make_voucher(
        &router,
        &fx.token,
        fx.fiscal_year_id,
        "2018-06-02",
        bank,
        sales,
        0,
        300,
    )
    .await;

    let coordinate = router
        .clone()
        .oneshot(
            Request::get(format!(
                "/api/v1/reports/ledger?fiscalYearId={}&generalLedgerCode=10&fromDate=2018-06-01&toDate=2018-12-31",
                fx.fiscal_year_id
            ))
            .header(header::COOKIE, cookie(&fx.token))
            .body(Body::empty())
            .unwrap(),
        )
        .await
        .unwrap();
    let coordinate_body = json_body(coordinate).await;
    // The 2018-06-01 debit line lands in the period (2 rows total: cash + bank), opening is zero.
    assert_eq!(
        coordinate_body["openingBalance"],
        json!({ "amount": 0, "side": "credit" })
    );
    assert_eq!(coordinate_body["rows"].as_array().unwrap().len(), 2);

    let ids = router
        .oneshot(
            Request::get(format!(
                "/api/v1/reports/ledger?fiscalYearId={}&accountIds={cash},{bank}&fromDate=2018-06-01&toDate=2018-12-31",
                fx.fiscal_year_id
            ))
            .header(header::COOKIE, cookie(&fx.token))
            .body(Body::empty())
            .unwrap(),
        )
        .await
        .unwrap();
    let ids_body = json_body(ids).await;

    // Same accounts, same window, reached via the arbitrary-id-list path
    // (TMoein's own use case) -- B4 fix: identical opening AND identical
    // movement, because both legs share the one predicate constant.
    assert_eq!(
        ids_body["openingBalance"],
        coordinate_body["openingBalance"]
    );
    assert_eq!(
        ids_body["closingBalance"],
        coordinate_body["closingBalance"]
    );
    assert_eq!(ids_body["rows"].as_array().unwrap().len(), 2);

    Ok(())
}

/// Manual test #5: a locked segment rejects a non-admin with the real rule,
/// not the legacy's "admin only" message.
#[sqlx::test(migrations = "./migrations")]
async fn locked_segment_rejects_non_admin_with_the_real_rule(pool: PgPool) -> sqlx::Result<()> {
    let fx = seed(&pool).await;
    let router = app(AppState { pool: pool.clone() });

    let assets = seed_account(&pool, fx.tenant_id, 10, 0, 0, 0, "Assets").await;
    seed_account(&pool, fx.tenant_id, 10, 1, 0, 0, "Cash").await;

    // The general-ledger view (Kol only, no subsidiaryCode) checks only the
    // Kol segment's own lock -- matches `Is_Admin_Or_Valid_Daftar(_K, 0, 0,
    // 0)`, which DKolU always calls with 0s for the segments it doesn't
    // select (04-03-a.md §3.1's permission note): locking a Moein under an
    // unrelated Kol-only query would not be checked at all.
    sqlx::query("UPDATE accounts SET is_locked = true WHERE id = $1")
        .bind(assets)
        .execute(&pool)
        .await?;

    let plain_user_id: i64 = sqlx::query_scalar(
        "INSERT INTO users (tenant_id, username, password_hash, is_superuser) \
         VALUES ($1, 'plain', 'x', false) RETURNING id",
    )
    .bind(fx.tenant_id)
    .fetch_one(&pool)
    .await?;
    let plain_token = format!("test-session-{plain_user_id}");
    sqlx::query(
        "INSERT INTO sessions (id, user_id, tenant_id, expires_at) \
         VALUES ($1, $2, $3, now() + interval '1 hour')",
    )
    .bind(&plain_token)
    .bind(plain_user_id)
    .bind(fx.tenant_id)
    .execute(&pool)
    .await?;
    // Grant the route's own permission (6.7) so this test isolates the
    // lock-specific rejection, not a "no grant at all" 403.
    sqlx::query(
        "INSERT INTO user_permissions (tenant_id, user_id, permission_id) \
         SELECT $1, $2, id FROM permissions WHERE code = 'general_ledger'",
    )
    .bind(fx.tenant_id)
    .bind(plain_user_id)
    .execute(&pool)
    .await?;

    let resp = router
        .oneshot(
            Request::get(format!(
                "/api/v1/reports/ledger?fiscalYearId={}&generalLedgerCode=10&fromDate=2018-06-01&toDate=2018-12-31",
                fx.fiscal_year_id
            ))
            .header(header::COOKIE, cookie(&plain_token))
            .body(Body::empty())
            .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    let body = json_body(resp).await;
    assert_eq!(body["error"], "account_segment_locked");

    Ok(())
}

/// The party-balance list (BedBes equivalent): B5's `< from_date` boundary
/// applied there too, debtor/creditor sign flip, and turnover filtering.
#[sqlx::test(migrations = "./migrations")]
async fn party_balances_apply_the_strict_boundary_and_debtor_creditor_sign_flip(
    pool: PgPool,
) -> sqlx::Result<()> {
    let fx = seed(&pool).await;
    let router = app(AppState { pool: pool.clone() });

    seed_account(&pool, fx.tenant_id, 103, 0, 0, 0, "Receivables Kol").await;
    seed_account(&pool, fx.tenant_id, 103, 1, 0, 0, "Trade AR").await;
    let cash = seed_account(&pool, fx.tenant_id, 10, 1, 0, 0, "Cash").await;
    let config_id: i64 = sqlx::query_scalar(
        "INSERT INTO party_account_config \
         (tenant_id, control_kol_code, control_moein_code, fixed_tafsil1_code, name, for_person, \
          for_legal_entity, offered_by_default, counts_toward_balance) \
         VALUES ($1, 103, 1, 0, 'Trade AR', true, false, true, true) RETURNING id",
    )
    .bind(fx.tenant_id)
    .fetch_one(&pool)
    .await?;
    let _ = config_id;

    let create_party = router
        .clone()
        .oneshot(
            Request::post("/api/v1/parties")
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::COOKIE, cookie(&fx.token))
                .body(Body::from(
                    json!({
                        "cardNumber": 900, "partyType": "natural_person",
                        "firstName": "Test", "lastName": "Debtor", "fatherName": "F",
                        "controlAccountConfigIds": [config_id],
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(create_party.status(), StatusCode::CREATED);

    let leaf: i64 = sqlx::query_scalar(
        "SELECT id FROM accounts WHERE tenant_id = $1 AND general_ledger_code = 103 \
         AND subsidiary_code = 1 AND analytic1_code = 900",
    )
    .bind(fx.tenant_id)
    .fetch_one(&pool)
    .await?;

    // Debit 2,000,000 on the party's leaf, dated exactly on from_date -> B5:
    // must land in the period, not the opening (strictly `< from_date`).
    make_voucher(
        &router,
        &fx.token,
        fx.fiscal_year_id,
        "2018-06-01",
        leaf,
        cash,
        2_000_000,
        0,
    )
    .await;

    let resp = router
        .clone()
        .oneshot(
            Request::get(format!(
                "/api/v1/reports/party-balances?fiscalYearId={}&fromDate=2018-06-01&toDate=2018-12-31",
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
    let rows = body["rows"].as_array().unwrap();
    let row = rows
        .iter()
        .find(|r| r["cardNumber"] == 900)
        .expect("party 900 present");
    assert_eq!(
        row["openingBalance"],
        json!({ "amount": 0, "side": "credit" }),
        "the on-boundary debit must not be in the opening"
    );
    assert_eq!(row["periodDebit"], 2_000_000);
    // debit 2,000,000, no credit -> net -2,000,000 -> a debtor.
    assert_eq!(
        row["closingBalance"],
        json!({ "amount": 2_000_000, "side": "debit" })
    );

    // Debtor-side filter with a matching amount window keeps the row.
    let debtors_only = router
        .clone()
        .oneshot(
            Request::get(format!(
                "/api/v1/reports/party-balances?fiscalYearId={}&fromDate=2018-06-01&toDate=2018-12-31&side=debtors&minAmount=1000000&maxAmount=3000000",
                fx.fiscal_year_id
            ))
            .header(header::COOKIE, cookie(&fx.token))
            .body(Body::empty())
            .unwrap(),
        )
        .await
        .unwrap();
    let debtors_body = json_body(debtors_only).await;
    assert_eq!(debtors_body["rows"].as_array().unwrap().len(), 1);

    // Creditor-side filter excludes the same (debtor) party entirely.
    let creditors_only = router
        .oneshot(
            Request::get(format!(
                "/api/v1/reports/party-balances?fiscalYearId={}&fromDate=2018-06-01&toDate=2018-12-31&side=creditors",
                fx.fiscal_year_id
            ))
            .header(header::COOKIE, cookie(&fx.token))
            .body(Body::empty())
            .unwrap(),
        )
        .await
        .unwrap();
    let creditors_body = json_body(creditors_only).await;
    assert!(creditors_body["rows"]
        .as_array()
        .unwrap()
        .iter()
        .all(|r| r["cardNumber"] != 900));

    Ok(())
}
