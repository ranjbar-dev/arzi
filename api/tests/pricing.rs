//! Automated version of step 5.5's manual test (docs/phase-5-inventory.md §5.5): a percentage
//! discount computed with correct rounding, an overridden line price that is never silently reset
//! to the item's list price, and the dual-entry-mode ambiguity rejection.

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
    document_id: i64,
    /// The item's list/sale price -- deliberately distinctive so a "silently reset to list price"
    /// bug would be unmistakable in an assertion.
    sale_price: i64,
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
    let sale_price = 777777;
    let resp = req(
        &router, "POST", "/api/v1/items", &token,
        json!({ "code": 1, "name": "Pistachio", "unitOfMeasureId": uom_id, "salePrice": sale_price }),
    )
    .await;
    let item_id = json_body(resp).await["id"].as_i64().unwrap();

    let resp = req(
        &router, "POST", "/api/v1/inventory-documents", &token,
        json!({
            "fiscalYearId": fiscal_year_id, "documentType": "receipt", "documentDate": "2027-05-01",
            "warehouseId": warehouse_id, "counterpartyAccountId": counterparty_id,
        }),
    )
    .await;
    let document_id = json_body(resp).await["id"].as_i64().unwrap();

    Fixture { token, document_id, sale_price, item_id }
}

/// Manual test #1: a percentage discount is computed correctly rounded, not truncated.
#[sqlx::test(migrations = "./migrations")]
async fn percentage_discount_rounds_correctly(pool: PgPool) -> sqlx::Result<()> {
    let f = setup(&pool).await;
    let router = app(AppState { pool: pool.clone() });

    // gross = 10 * 1000 = 10,000; 3.335% -> 333.5 -> rounds to 334 (trunc would give 333).
    let resp = req(
        &router, "POST", &format!("/api/v1/inventory-documents/{}/lines", f.document_id), &f.token,
        json!({ "itemId": f.item_id, "quantity": "10", "unitPrice": 1000, "discountPercent": "3.335" }),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::CREATED);
    let line_id = json_body(resp).await["id"].as_i64().unwrap();

    let resp = req(&router, "GET", &format!("/api/v1/inventory-documents/{}", f.document_id), &f.token, Value::Null).await;
    let body = json_body(resp).await;
    let line = body["lines"].as_array().unwrap().iter().find(|l| l["id"] == line_id).unwrap();
    assert_eq!(line["discountAmount"], 334);
    assert_eq!(line["totalAmount"], 10000 - 334);

    // Both entry modes at once is rejected as ambiguous.
    let resp = req(
        &router, "POST", &format!("/api/v1/inventory-documents/{}/lines", f.document_id), &f.token,
        json!({ "itemId": f.item_id, "quantity": "10", "unitPrice": 1000, "discountAmount": 100, "discountPercent": "5" }),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

    // Out-of-range percentage rejected.
    let resp = req(
        &router, "POST", &format!("/api/v1/inventory-documents/{}/lines", f.document_id), &f.token,
        json!({ "itemId": f.item_id, "quantity": "10", "unitPrice": 1000, "discountPercent": "150" }),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    Ok(())
}

/// Manual test #2: overriding the line price away from the item's list price is never silently
/// reset back to it, on either create or a later edit.
#[sqlx::test(migrations = "./migrations")]
async fn overridden_price_is_never_reset_to_list_price(pool: PgPool) -> sqlx::Result<()> {
    let f = setup(&pool).await;
    let router = app(AppState { pool: pool.clone() });
    let overridden_price = 12345; // deliberately far from f.sale_price (777,777)
    assert_ne!(overridden_price, f.sale_price);

    let resp = req(
        &router, "POST", &format!("/api/v1/inventory-documents/{}/lines", f.document_id), &f.token,
        json!({ "itemId": f.item_id, "quantity": "1", "unitPrice": overridden_price }),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::CREATED);
    let line_id = json_body(resp).await["id"].as_i64().unwrap();

    let resp = req(&router, "GET", &format!("/api/v1/inventory-documents/{}", f.document_id), &f.token, Value::Null).await;
    let body = json_body(resp).await;
    let line = body["lines"].as_array().unwrap().iter().find(|l| l["id"] == line_id).unwrap();
    assert_eq!(line["unitPrice"], overridden_price);
    assert_ne!(line["unitPrice"], f.sale_price);

    // Editing the line (e.g. changing quantity) must not silently reset the price either.
    let resp = req(
        &router, "PUT", &format!("/api/v1/inventory-documents/{}/lines/{line_id}", f.document_id), &f.token,
        json!({ "itemId": f.item_id, "quantity": "2", "unitPrice": overridden_price }),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);
    let resp = req(&router, "GET", &format!("/api/v1/inventory-documents/{}", f.document_id), &f.token, Value::Null).await;
    let body = json_body(resp).await;
    let line = body["lines"].as_array().unwrap().iter().find(|l| l["id"] == line_id).unwrap();
    assert_eq!(line["unitPrice"], overridden_price);
    Ok(())
}
