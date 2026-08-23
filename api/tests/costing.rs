//! Automated version of step 5.4's manual test (docs/phase-5-inventory.md §5.4): the worked
//! weighted-average-of-purchases example, zero-purchases default (never `sale_price`), and the
//! exclude-current-document behaviour shared with 5.3's on-hand formula.

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
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

async fn req(router: &axum::Router, method: &str, path: &str, token: &str, body: Value) -> axum::response::Response {
    let mut builder = Request::builder().method(method).uri(path).header(header::COOKIE, cookie(token));
    let b = if body.is_null() {
        Body::empty()
    } else {
        builder = builder.header(header::CONTENT_TYPE, "application/json");
        Body::from(body.to_string())
    };
    router.clone().oneshot(builder.body(b).unwrap()).await.unwrap()
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
    fiscal_year_id: i64,
    warehouse_id: i64,
    counterparty_id: i64,
    item_id: i64,
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
        &router, "POST", "/api/v1/warehouses", &token,
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

    let resp = req(&router, "POST", "/api/v1/units-of-measure", &token, json!({ "name": "kg" })).await;
    let uom_id = json_body(resp).await["id"].as_i64().unwrap();
    let resp = req(
        &router, "POST", "/api/v1/items", &token,
        json!({ "code": 1, "name": "Pistachio", "unitOfMeasureId": uom_id, "salePrice": 999999 }),
    )
    .await;
    let item_id = json_body(resp).await["id"].as_i64().unwrap();

    Fixture { token, fiscal_year_id, warehouse_id, counterparty_id, item_id }
}

async fn post_receipt(router: &axum::Router, f: &Fixture, date: &str, quantity: &str, unit_price: i64) -> i64 {
    let resp = req(
        router, "POST", "/api/v1/inventory-documents", &f.token,
        json!({
            "fiscalYearId": f.fiscal_year_id, "documentType": "receipt", "documentDate": date,
            "warehouseId": f.warehouse_id, "counterpartyAccountId": f.counterparty_id,
        }),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::CREATED);
    let doc_id = json_body(resp).await["id"].as_i64().unwrap();
    let resp = req(
        router, "POST", &format!("/api/v1/inventory-documents/{doc_id}/lines"), &f.token,
        json!({ "itemId": f.item_id, "quantity": quantity, "unitPrice": unit_price }),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::CREATED);
    doc_id
}

async fn average_cost(router: &axum::Router, f: &Fixture, as_of: &str, exclude: Option<i64>) -> Value {
    let mut url = format!(
        "/api/v1/items/{}/average-cost?fiscalYearId={}&asOfDate={as_of}",
        f.item_id, f.fiscal_year_id
    );
    if let Some(id) = exclude {
        url.push_str(&format!("&excludeDocumentId={id}"));
    }
    let resp = req(router, "GET", &url, &f.token, Value::Null).await;
    assert_eq!(resp.status(), StatusCode::OK);
    json_body(resp).await
}

/// Manual test #1: 100@50,000 + 60@65,000 + 40@72,500 -> exactly 59,000 rial/kg.
#[sqlx::test(migrations = "./migrations")]
async fn worked_example_average_cost(pool: PgPool) -> sqlx::Result<()> {
    let f = setup(&pool).await;
    let router = app(AppState { pool: pool.clone() });

    post_receipt(&router, &f, "2027-04-01", "100", 50000).await;
    post_receipt(&router, &f, "2027-04-05", "60", 65000).await;
    post_receipt(&router, &f, "2027-04-10", "40", 72500).await;

    let body = average_cost(&router, &f, "2027-04-15", None).await;
    assert_eq!(body["averageCost"], 59000);
    assert_eq!(body["purchaseQuantity"], "200");
    Ok(())
}

/// Manual test #2 (structural): with no purchases at all, the suggestion is 0 — never the item's
/// sale price (999,999 in the fixture), closing 05-06-a.md §6.1's "pre-filled with the selling
/// price on a goods receipt" bug at the API boundary.
#[sqlx::test(migrations = "./migrations")]
async fn zero_purchases_gives_zero_never_sale_price(pool: PgPool) -> sqlx::Result<()> {
    let f = setup(&pool).await;
    let router = app(AppState { pool: pool.clone() });

    let body = average_cost(&router, &f, "2027-04-15", None).await;
    assert_eq!(body["averageCost"], 0);
    assert_ne!(body["averageCost"], 999999);
    Ok(())
}

/// The document currently being edited excludes its own lines from the average, same convention
/// as 5.3's on-hand exclude-self-document behaviour.
#[sqlx::test(migrations = "./migrations")]
async fn excludes_the_document_being_edited(pool: PgPool) -> sqlx::Result<()> {
    let f = setup(&pool).await;
    let router = app(AppState { pool: pool.clone() });

    post_receipt(&router, &f, "2027-04-01", "100", 50000).await;
    let editing_doc = post_receipt(&router, &f, "2027-04-05", "60", 65000).await;

    let body = average_cost(&router, &f, "2027-04-15", Some(editing_doc)).await;
    assert_eq!(body["averageCost"], 50000); // only the first purchase counts
    assert_eq!(body["purchaseQuantity"], "100");
    Ok(())
}

/// Sales, sales returns and purchase returns never enter the average (§6.8: purchases only).
#[sqlx::test(migrations = "./migrations")]
async fn non_purchase_types_excluded_from_average(pool: PgPool) -> sqlx::Result<()> {
    let f = setup(&pool).await;
    let router = app(AppState { pool: pool.clone() });

    post_receipt(&router, &f, "2027-04-01", "100", 50000).await;

    // An issue (sale) at a wildly different price must not move the average.
    let resp = req(
        &router, "POST", "/api/v1/inventory-documents", &f.token,
        json!({
            "fiscalYearId": f.fiscal_year_id, "documentType": "issue", "documentDate": "2027-04-05",
            "warehouseId": f.warehouse_id, "counterpartyAccountId": f.counterparty_id,
        }),
    )
    .await;
    let issue_id = json_body(resp).await["id"].as_i64().unwrap();
    req(
        &router, "POST", &format!("/api/v1/inventory-documents/{issue_id}/lines"), &f.token,
        json!({ "itemId": f.item_id, "quantity": "10", "unitPrice": 999999 }),
    )
    .await;

    let body = average_cost(&router, &f, "2027-04-15", None).await;
    assert_eq!(body["averageCost"], 50000); // unchanged by the sale
    Ok(())
}
