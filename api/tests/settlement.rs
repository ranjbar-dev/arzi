//! Automated version of step 5.7's manual test (docs/phase-5-inventory.md §5.7): a deposit slip
//! and two cheques attached to an invoice, sorted by date, with a real computed outstanding
//! figure — including the over-settlement case the legacy screen never surfaced.

use api::{app, AppState};
use axum::{
    body::Body,
    http::{header, Request, StatusCode},
};
use serde_json::{json, Value};
use sqlx::PgPool;
use tower::ServiceExt;

async fn seed_tenant(pool: &PgPool) -> i64 {
    sqlx::query_scalar("INSERT INTO tenants (slug, name) VALUES ('acme', 'Acme') RETURNING id")
        .fetch_one(pool)
        .await
        .unwrap()
}

async fn make_superuser(pool: &PgPool, tenant_id: i64) -> String {
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
        "INSERT INTO sessions (id, user_id, tenant_id, expires_at) VALUES ($1, $2, $3, now() + interval '1 hour')",
    )
    .bind(&token)
    .bind(user_id)
    .bind(tenant_id)
    .execute(pool)
    .await
    .unwrap();
    token
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

async fn req(
    router: &axum::Router,
    method: &str,
    path: &str,
    token: &str,
    body: Value,
) -> axum::response::Response {
    let mut builder = Request::builder()
        .method(method)
        .uri(path)
        .header(header::COOKIE, cookie(token));
    let b = if body.is_null() {
        Body::empty()
    } else {
        builder = builder.header(header::CONTENT_TYPE, "application/json");
        Body::from(body.to_string())
    };
    router
        .clone()
        .oneshot(builder.body(b).unwrap())
        .await
        .unwrap()
}

async fn seed_fiscal_year(pool: &PgPool, tenant_id: i64) -> i64 {
    sqlx::query_scalar(
        "INSERT INTO fiscal_years (tenant_id, year, start_date, end_date, is_active) \
         VALUES ($1, 1406, '2027-03-21', '2028-03-19', true) RETURNING id",
    )
    .bind(tenant_id)
    .fetch_one(pool)
    .await
    .unwrap()
}

async fn seed_leaf_account(pool: &PgPool, tenant_id: i64, gl: i32, sub: i32, name: &str) -> i64 {
    sqlx::query_scalar(
        "INSERT INTO accounts (tenant_id, general_ledger_code, subsidiary_code, analytic1_code, \
         analytic2_code, name) VALUES ($1, $2, $3, 0, 0, $4) RETURNING id",
    )
    .bind(tenant_id)
    .bind(gl)
    .bind(sub)
    .bind(name)
    .fetch_one(pool)
    .await
    .unwrap()
}

struct Fixture {
    token: String,
    document_id: i64,
    /// A leaf account playable as payer / bank / notes-receivable in any of these tests.
    misc_account_id: i64,
}

async fn setup(pool: &PgPool) -> Fixture {
    let tenant_id = seed_tenant(pool).await;
    let token = make_superuser(pool, tenant_id).await;
    let router = app(AppState { pool: pool.clone() });
    let fiscal_year_id = seed_fiscal_year(pool, tenant_id).await;

    let accounts = [
        seed_leaf_account(pool, tenant_id, 900, 1, "Purchases").await,
        seed_leaf_account(pool, tenant_id, 900, 2, "Purchase returns").await,
        seed_leaf_account(pool, tenant_id, 901, 1, "Sales").await,
        seed_leaf_account(pool, tenant_id, 901, 2, "Sales returns").await,
        seed_leaf_account(pool, tenant_id, 902, 1, "Discounts").await,
        seed_leaf_account(pool, tenant_id, 902, 2, "VAT").await,
    ];
    let resp = req(
        &router,
        "POST",
        "/api/v1/warehouses",
        &token,
        json!({
            "name": "Main", "vatRatePct": "9",
            "purchaseAccountId": accounts[0], "purchaseReturnAccountId": accounts[1],
            "salesAccountId": accounts[2], "salesReturnAccountId": accounts[3],
            "discountAccountId": accounts[4], "vatAccountId": accounts[5],
        }),
    )
    .await;
    let warehouse_id = json_body(resp).await["id"].as_i64().unwrap();
    let counterparty_id = seed_leaf_account(pool, tenant_id, 103, 1, "Trade AR").await;
    let misc_account_id = seed_leaf_account(pool, tenant_id, 104, 1, "Misc").await;

    let resp = req(
        &router,
        "POST",
        "/api/v1/units-of-measure",
        &token,
        json!({ "name": "kg" }),
    )
    .await;
    let uom_id = json_body(resp).await["id"].as_i64().unwrap();
    let resp = req(
        &router,
        "POST",
        "/api/v1/items",
        &token,
        json!({ "code": 1, "name": "Item", "unitOfMeasureId": uom_id, "salePrice": 1000 }),
    )
    .await;
    let item_id = json_body(resp).await["id"].as_i64().unwrap();

    let resp = req(
        &router,
        "POST",
        "/api/v1/inventory-documents",
        &token,
        json!({
            "fiscalYearId": fiscal_year_id, "documentType": "issue", "documentDate": "2027-05-01",
            "warehouseId": warehouse_id, "counterpartyAccountId": counterparty_id,
        }),
    )
    .await;
    let document_id = json_body(resp).await["id"].as_i64().unwrap();
    req(
        &router,
        "POST",
        &format!("/api/v1/inventory-documents/{document_id}/lines"),
        &token,
        json!({ "itemId": item_id, "quantity": "10", "unitPrice": 1_000_000 }), // invoice total 10,000,000
    )
    .await;

    Fixture {
        token,
        document_id,
        misc_account_id,
    }
}

async fn attach_cheque(
    router: &axum::Router,
    f: &Fixture,
    fiscal_year_id: i64,
    date: &str,
    amount: i64,
) {
    let resp = req(
        router,
        "POST",
        "/api/v1/received-cheques",
        &f.token,
        json!({
            "fiscalYearId": fiscal_year_id, "receivedOn": date, "dueDate": date, "amount": amount,
            "description": "settlement test cheque", "payerAccountId": f.misc_account_id,
            "notesReceivableAccountId": f.misc_account_id, "sourceDocumentId": f.document_id,
        }),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::CREATED, "cheque create failed");
}

async fn attach_deposit_slip(
    router: &axum::Router,
    f: &Fixture,
    fiscal_year_id: i64,
    date: &str,
    amount: i64,
) {
    let resp = req(
        router,
        "POST",
        "/api/v1/deposit-slips",
        &f.token,
        json!({
            "fiscalYearId": fiscal_year_id, "slipDate": date, "amount": amount,
            "payerAccountId": f.misc_account_id, "bankAccountId": f.misc_account_id,
            "channel": "wire_transfer", "sourceDocumentId": f.document_id,
        }),
    )
    .await;
    assert_eq!(
        resp.status(),
        StatusCode::CREATED,
        "deposit slip create failed"
    );
}

async fn fiscal_year_id_of(pool: &PgPool) -> i64 {
    sqlx::query_scalar("SELECT id FROM fiscal_years LIMIT 1")
        .fetch_one(pool)
        .await
        .unwrap()
}

/// Manual test #1/#2: a deposit slip and two cheques attached, all three appear sorted by date,
/// with a computed outstanding figure.
#[sqlx::test(migrations = "./migrations")]
async fn attached_instruments_sorted_with_outstanding(pool: PgPool) -> sqlx::Result<()> {
    let f = setup(&pool).await;
    let router = app(AppState { pool: pool.clone() });
    let fiscal_year_id = fiscal_year_id_of(&pool).await;

    // Attach out of chronological order to prove the sort is real, not insertion order.
    attach_cheque(&router, &f, fiscal_year_id, "2027-05-10", 2_000_000).await;
    attach_deposit_slip(&router, &f, fiscal_year_id, "2027-05-03", 3_000_000).await;
    attach_cheque(&router, &f, fiscal_year_id, "2027-05-07", 1_000_000).await;

    let resp = req(
        &router,
        "GET",
        &format!("/api/v1/inventory-documents/{}/settlement", f.document_id),
        &f.token,
        Value::Null,
    )
    .await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body = json_body(resp).await;

    assert_eq!(body["invoiceTotal"], 10_000_000);
    assert_eq!(body["settledTotal"], 6_000_000);
    assert_eq!(body["outstandingAmount"], 4_000_000);

    let instruments = body["instruments"].as_array().unwrap();
    assert_eq!(instruments.len(), 3);
    let dates: Vec<&str> = instruments
        .iter()
        .map(|i| i["date"].as_str().unwrap())
        .collect();
    assert_eq!(dates, vec!["2027-05-03", "2027-05-07", "2027-05-10"]); // real date order
    assert_eq!(instruments[0]["kind"], "deposit_slip");
    assert_eq!(instruments[1]["kind"], "received_cheque");
    Ok(())
}

/// Manual test #3: over-settling is still permitted (no block), and the outstanding figure goes
/// negative rather than being silent about it.
#[sqlx::test(migrations = "./migrations")]
async fn over_settlement_permitted_and_visible(pool: PgPool) -> sqlx::Result<()> {
    let f = setup(&pool).await;
    let router = app(AppState { pool: pool.clone() });
    let fiscal_year_id = fiscal_year_id_of(&pool).await;

    // Invoice total is 10,000,000; attach instruments totalling 15,000,000.
    attach_cheque(&router, &f, fiscal_year_id, "2027-05-05", 8_000_000).await;
    attach_deposit_slip(&router, &f, fiscal_year_id, "2027-05-06", 7_000_000).await;

    let resp = req(
        &router,
        "GET",
        &format!("/api/v1/inventory-documents/{}/settlement", f.document_id),
        &f.token,
        Value::Null,
    )
    .await;
    assert_eq!(resp.status(), StatusCode::OK); // not blocked
    let body = json_body(resp).await;
    assert_eq!(body["settledTotal"], 15_000_000);
    assert_eq!(body["outstandingAmount"], -5_000_000); // clearly over-paid, not silent
    Ok(())
}

/// An instrument with no source link at all does not show up on any invoice's settlement view.
#[sqlx::test(migrations = "./migrations")]
async fn unlinked_instruments_do_not_appear(pool: PgPool) -> sqlx::Result<()> {
    let f = setup(&pool).await;
    let router = app(AppState { pool: pool.clone() });
    let fiscal_year_id = fiscal_year_id_of(&pool).await;

    // A cheque created with no sourceDocumentId at all.
    let resp = req(
        &router, "POST", "/api/v1/received-cheques", &f.token,
        json!({
            "fiscalYearId": fiscal_year_id, "receivedOn": "2027-05-05", "dueDate": "2027-05-05",
            "amount": 500_000, "description": "unrelated cheque", "payerAccountId": f.misc_account_id,
            "notesReceivableAccountId": f.misc_account_id,
        }),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::CREATED);

    let resp = req(
        &router,
        "GET",
        &format!("/api/v1/inventory-documents/{}/settlement", f.document_id),
        &f.token,
        Value::Null,
    )
    .await;
    let body = json_body(resp).await;
    assert_eq!(body["settledTotal"], 0);
    assert_eq!(body["outstandingAmount"], 10_000_000);
    assert_eq!(body["instruments"].as_array().unwrap().len(), 0);
    Ok(())
}
