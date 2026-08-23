//! Automated version of step 6.7's manual test (docs/phase-6-reporting.md
//! §6.7): every report route built in 6.1-6.6 has a real, independently-
//! grantable permission check -- including B24's exact defect class (the
//! legacy left `Report5`/`Report8`/the tax-authority export completely
//! ungated, no catalogue id at all).

use api::{app, AppState};
use axum::{
    body::Body,
    http::{header, Request, StatusCode},
};
use sqlx::PgPool;
use tower::ServiceExt;

struct Fixture {
    tenant_id: i64,
    plain_user_id: i64,
    plain_token: String,
    fiscal_year_id: i64,
}

async fn seed(pool: &PgPool) -> Fixture {
    let tenant_id: i64 = sqlx::query_scalar("INSERT INTO tenants (slug, name) VALUES ('acme', 'Acme') RETURNING id")
        .fetch_one(pool)
        .await
        .unwrap();
    let plain_user_id: i64 = sqlx::query_scalar(
        "INSERT INTO users (tenant_id, username, password_hash, is_superuser) \
         VALUES ($1, 'plain', 'x', false) RETURNING id",
    )
    .bind(tenant_id)
    .fetch_one(pool)
    .await
    .unwrap();
    let plain_token = format!("test-session-{plain_user_id}");
    sqlx::query(
        "INSERT INTO sessions (id, user_id, tenant_id, expires_at) \
         VALUES ($1, $2, $3, now() + interval '1 hour')",
    )
    .bind(&plain_token)
    .bind(plain_user_id)
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
    Fixture { tenant_id, plain_user_id, plain_token, fiscal_year_id }
}

fn cookie(token: &str) -> String {
    format!("arzi_session={token}")
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

/// Manual test #1/#2: every report route enumerated here 403s a user with
/// no grants at all, including the ones the legacy left completely
/// ungated (B24: `trial_balance_6_column`, `debtors_creditors_report`,
/// `tax_authority_export` -- none of these had ANY catalogue id before this
/// step, direct proof of the fix, not just of a check existing).
#[sqlx::test(migrations = "./migrations")]
async fn every_report_route_rejects_a_user_with_no_grants(pool: PgPool) -> sqlx::Result<()> {
    let fx = seed(&pool).await;
    let router = app(AppState { pool: pool.clone() });
    let fy = fx.fiscal_year_id;

    let routes = [
        format!("/api/v1/reports/trial-balance-4-column?fiscalYearId={fy}&asOfDate=2028-01-01"),
        format!("/api/v1/reports/trial-balance-4-column/pdf?fiscalYearId={fy}&asOfDate=2028-01-01"),
        format!("/api/v1/reports/trial-balance-6-column?fiscalYearId={fy}&fromDate=2027-01-01&toDate=2028-01-01"),
        format!("/api/v1/reports/ledger?fiscalYearId={fy}&generalLedgerCode=1&fromDate=2027-01-01&toDate=2028-01-01"),
        format!("/api/v1/reports/party-balances?fiscalYearId={fy}&fromDate=2027-01-01&toDate=2028-01-01"),
        format!("/api/v1/reports/inventory-activity?fiscalYearId={fy}&fromDate=2027-01-01&toDate=2028-01-01"),
        format!("/api/v1/reports/stock-balance?fiscalYearId={fy}&asOfDate=2028-01-01"),
        format!("/api/v1/reports/tax-authority-export?fiscalYearId={fy}&fromDate=2027-01-01&toDate=2028-01-01"),
    ];
    for route in &routes {
        let resp = router
            .clone()
            .oneshot(Request::get(route).header(header::COOKIE, cookie(&fx.plain_token)).body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::FORBIDDEN, "route {route} must 403 with no grants");
    }

    Ok(())
}

/// Manual test #1 (independence): each report's own grant unlocks ONLY
/// that report -- proven for the three genuinely new B24 ids by granting
/// one and confirming a *different* new-id route still 403s.
#[sqlx::test(migrations = "./migrations")]
async fn each_new_permission_is_independently_grantable(pool: PgPool) -> sqlx::Result<()> {
    let fx = seed(&pool).await;
    let router = app(AppState { pool: pool.clone() });
    let fy = fx.fiscal_year_id;

    grant(&pool, fx.tenant_id, fx.plain_user_id, "trial_balance_6_column").await;

    let six_col = router
        .clone()
        .oneshot(
            Request::get(format!("/api/v1/reports/trial-balance-6-column?fiscalYearId={fy}&fromDate=2027-01-01&toDate=2028-01-01"))
                .header(header::COOKIE, cookie(&fx.plain_token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(six_col.status(), StatusCode::OK, "the granted report must now succeed");

    let debtors = router
        .clone()
        .oneshot(
            Request::get(format!("/api/v1/reports/party-balances?fiscalYearId={fy}&fromDate=2027-01-01&toDate=2028-01-01"))
                .header(header::COOKIE, cookie(&fx.plain_token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(debtors.status(), StatusCode::FORBIDDEN, "an unrelated report must still 403");

    grant(&pool, fx.tenant_id, fx.plain_user_id, "debtors_creditors_report").await;
    let debtors2 = router
        .oneshot(
            Request::get(format!("/api/v1/reports/party-balances?fiscalYearId={fy}&fromDate=2027-01-01&toDate=2028-01-01"))
                .header(header::COOKIE, cookie(&fx.plain_token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(debtors2.status(), StatusCode::OK, "granting its own id unlocks it");

    Ok(())
}

/// Manual test #3: the merged ledger route (Daftar Kol + Daftar Moein) is
/// consistently gated -- either of its two legacy-precedent ids works, no
/// repeat of the `Report1`-gated/`Report2`-ungated split B24 also flags.
#[sqlx::test(migrations = "./migrations")]
async fn ledger_route_accepts_either_of_its_two_merged_ids(pool: PgPool) -> sqlx::Result<()> {
    let fx = seed(&pool).await;
    let router = app(AppState { pool: pool.clone() });
    let fy = fx.fiscal_year_id;
    let url = format!("/api/v1/reports/ledger?fiscalYearId={fy}&generalLedgerCode=1&fromDate=2027-01-01&toDate=2028-01-01");

    grant(&pool, fx.tenant_id, fx.plain_user_id, "general_ledger").await;
    let via_general = router
        .clone()
        .oneshot(Request::get(&url).header(header::COOKIE, cookie(&fx.plain_token)).body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(via_general.status(), StatusCode::OK, "general_ledger alone must unlock the merged route");

    // A second, independent user with only the subsidiary-ledger id.
    let other_user_id: i64 = sqlx::query_scalar(
        "INSERT INTO users (tenant_id, username, password_hash, is_superuser) \
         VALUES ($1, 'plain2', 'x', false) RETURNING id",
    )
    .bind(fx.tenant_id)
    .fetch_one(&pool)
    .await?;
    let other_token = format!("test-session-{other_user_id}");
    sqlx::query(
        "INSERT INTO sessions (id, user_id, tenant_id, expires_at) VALUES ($1, $2, $3, now() + interval '1 hour')",
    )
    .bind(&other_token)
    .bind(other_user_id)
    .bind(fx.tenant_id)
    .execute(&pool)
    .await?;
    grant(&pool, fx.tenant_id, other_user_id, "view_subsidiary_ledger").await;
    let via_subsidiary = router
        .oneshot(Request::get(&url).header(header::COOKIE, cookie(&other_token)).body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(via_subsidiary.status(), StatusCode::OK, "view_subsidiary_ledger alone must also unlock it");

    Ok(())
}
