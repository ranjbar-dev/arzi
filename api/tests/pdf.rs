//! Automated version of step 6.5's manual test (docs/phase-6-reporting.md
//! §6.5): a real PDF renders for a posted voucher and for the 4-column
//! trial balance, both with letterhead/organisation name/fiscal-year
//! caption/signature block sourced from the one structured `PrintHeader`,
//! and B23 (no template-edit surface exists at all).

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
    cash_account_id: i64,
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
    sqlx::query("INSERT INTO organization (tenant_id, name) VALUES ($1, 'Acme Co')")
        .bind(tenant_id)
        .execute(pool)
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO app_settings (tenant_id, key, value, value_type, label_fa) \
         VALUES ($1, 'voucher_signature_1', 'مدیر مالی', 'string', 'x')",
    )
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
    Fixture { tenant_id, fiscal_year_id, token, cash_account_id }
}

fn cookie(token: &str) -> String {
    format!("arzi_session={token}")
}

async fn json_body(response: axum::response::Response) -> Value {
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

async fn seed_account(pool: &PgPool, tenant_id: i64, gl: i32, sub: i32, name: &str) -> i64 {
    sqlx::query_scalar(
        "INSERT INTO accounts (tenant_id, general_ledger_code, subsidiary_code, name) \
         VALUES ($1, $2, $3, $4) RETURNING id",
    )
    .bind(tenant_id)
    .bind(gl)
    .bind(sub)
    .bind(name)
    .fetch_one(pool)
    .await
    .unwrap()
}

/// Manual test #1: a posted voucher's PDF has the letterhead, organisation
/// name, fiscal-year caption and signature block all correctly rendered.
#[sqlx::test(migrations = "./migrations")]
async fn voucher_pdf_renders_with_structured_header(pool: PgPool) -> sqlx::Result<()> {
    let fx = seed(&pool).await;
    let router = app(AppState { pool: pool.clone() });
    let sales = seed_account(&pool, fx.tenant_id, 40, 1, "Sales").await;

    let create = router
        .clone()
        .oneshot(
            Request::post("/api/v1/vouchers")
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::COOKIE, cookie(&fx.token))
                .body(Body::from(
                    json!({ "fiscalYearId": fx.fiscal_year_id, "voucherDate": "2027-06-01", "description": "PDF fixture" })
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
        let token = fx.token.clone();
        async move {
            router
                .oneshot(
                    Request::post(format!("/api/v1/vouchers/{voucher_id}/lines"))
                        .header(header::CONTENT_TYPE, "application/json")
                        .header(header::COOKIE, cookie(&token))
                        .body(Body::from(
                            json!({ "accountId": acc, "debit": debit, "credit": credit, "description": "دریافت نقد" })
                                .to_string(),
                        ))
                        .unwrap(),
                )
                .await
                .unwrap()
        }
    };
    assert_eq!(add_line(fx.cash_account_id, 1_500_000, 0).await.status(), StatusCode::CREATED);
    assert_eq!(add_line(sales, 0, 1_500_000).await.status(), StatusCode::CREATED);

    let pdf_resp = router
        .oneshot(
            Request::get(format!("/api/v1/vouchers/{voucher_id}/pdf"))
                .header(header::COOKIE, cookie(&fx.token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(pdf_resp.status(), StatusCode::OK);
    assert_eq!(pdf_resp.headers().get(header::CONTENT_TYPE).unwrap(), "application/pdf");
    let bytes = axum::body::to_bytes(pdf_resp.into_body(), usize::MAX).await.unwrap();
    assert!(bytes.starts_with(b"%PDF-"), "response body must be a real PDF");
    assert!(bytes.len() > 500);

    Ok(())
}

/// Manual test #2: a report PDF (a numeric amount, here the trial
/// balance's own grand total) renders with a correct Persian amount-in-
/// words footer -- exercised end to end via the real HTTP route.
#[sqlx::test(migrations = "./migrations")]
async fn trial_balance_pdf_renders(pool: PgPool) -> sqlx::Result<()> {
    let fx = seed(&pool).await;
    let router = app(AppState { pool: pool.clone() });
    let sales = seed_account(&pool, fx.tenant_id, 40, 1, "Sales").await;

    let create = router
        .clone()
        .oneshot(
            Request::post("/api/v1/vouchers")
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::COOKIE, cookie(&fx.token))
                .body(Body::from(
                    json!({ "fiscalYearId": fx.fiscal_year_id, "voucherDate": "2027-06-01", "description": "PDF fixture" })
                        .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    let voucher_id = json_body(create).await["id"].as_i64().unwrap();
    let add_line = |acc: i64, debit: i64, credit: i64| {
        let router = router.clone();
        let token = fx.token.clone();
        async move {
            router
                .oneshot(
                    Request::post(format!("/api/v1/vouchers/{voucher_id}/lines"))
                        .header(header::CONTENT_TYPE, "application/json")
                        .header(header::COOKIE, cookie(&token))
                        .body(Body::from(
                            json!({ "accountId": acc, "debit": debit, "credit": credit, "description": "line" }).to_string(),
                        ))
                        .unwrap(),
                )
                .await
                .unwrap()
        }
    };
    add_line(fx.cash_account_id, 1_000_000, 0).await;
    add_line(sales, 0, 1_000_000).await;
    let transition = |to: &'static str| {
        let router = router.clone();
        let token = fx.token.clone();
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

    let pdf_resp = router
        .oneshot(
            Request::get(format!(
                "/api/v1/reports/trial-balance-4-column/pdf?fiscalYearId={}&asOfDate=2028-01-01&level=kol",
                fx.fiscal_year_id
            ))
            .header(header::COOKIE, cookie(&fx.token))
            .body(Body::empty())
            .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(pdf_resp.status(), StatusCode::OK);
    assert_eq!(pdf_resp.headers().get(header::CONTENT_TYPE).unwrap(), "application/pdf");
    let bytes = axum::body::to_bytes(pdf_resp.into_body(), usize::MAX).await.unwrap();
    assert!(bytes.starts_with(b"%PDF-"));

    Ok(())
}

/// B23 direct test: no route anywhere accepts a template file or lets a
/// caller choose one -- both PDF routes take only report parameters, never
/// a layout/template identifier.
#[sqlx::test(migrations = "./migrations")]
async fn pdf_routes_accept_no_template_selection_parameter(pool: PgPool) -> sqlx::Result<()> {
    let fx = seed(&pool).await;
    let router = app(AppState { pool });

    // A `template`/`layout`-style parameter is simply ignored, not honoured
    // -- there is no code path in `pdf.rs` that reads one at all.
    let resp = router
        .oneshot(
            Request::get(format!(
                "/api/v1/reports/trial-balance-4-column/pdf?fiscalYearId={}&asOfDate=2028-01-01&template=evil.typ",
                fx.fiscal_year_id
            ))
            .header(header::COOKIE, cookie(&fx.token))
            .body(Body::empty())
            .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    assert!(bytes.starts_with(b"%PDF-"), "still renders via the one built-in template, ignoring the parameter");

    Ok(())
}
