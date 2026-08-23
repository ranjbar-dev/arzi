//! Automated version of step 5.3's manual test (docs/phase-5-inventory.md §5.3): the canonical
//! on-hand formula, date-windowed opening balance, exclude-self-document, and a stock card whose
//! running balance matches on-hand at every point. Also closes 5.1's manual test #4 (a real
//! low-stock alert), deferred there until this step's on-hand query existed.

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
        json!({ "code": 1, "name": "Pistachio", "unitOfMeasureId": uom_id, "salePrice": 100000, "minStock": 50 }),
    )
    .await;
    let item_id = json_body(resp).await["id"].as_i64().unwrap();

    Fixture { token, fiscal_year_id, warehouse_id, counterparty_id, item_id }
}

/// Creates a draft document of `document_type` on `date` with one line of `quantity` for the
/// fixture's item, and returns the document id.
async fn post_movement(router: &axum::Router, f: &Fixture, document_type: &str, date: &str, quantity: &str) -> i64 {
    let resp = req(
        router, "POST", "/api/v1/inventory-documents", &f.token,
        json!({
            "fiscalYearId": f.fiscal_year_id, "documentType": document_type, "documentDate": date,
            "warehouseId": f.warehouse_id, "counterpartyAccountId": f.counterparty_id,
        }),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::CREATED, "create {document_type} on {date}");
    let doc_id = json_body(resp).await["id"].as_i64().unwrap();

    let resp = req(
        router, "POST", &format!("/api/v1/inventory-documents/{doc_id}/lines"), &f.token,
        json!({ "itemId": f.item_id, "quantity": quantity, "unitPrice": 10000 }),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::CREATED, "add line to {document_type}");
    doc_id
}

async fn on_hand(router: &axum::Router, f: &Fixture, as_of: &str) -> Value {
    let resp = req(
        router, "GET",
        &format!("/api/v1/items/{}/on-hand?fiscalYearId={}&asOfDate={as_of}", f.item_id, f.fiscal_year_id),
        &f.token, Value::Null,
    )
    .await;
    assert_eq!(resp.status(), StatusCode::OK);
    json_body(resp).await
}

/// Manual test #1: receipt 100, sale 30, sales return 5 -> on-hand = 75.
#[sqlx::test(migrations = "./migrations")]
async fn canonical_on_hand_formula(pool: PgPool) -> sqlx::Result<()> {
    let f = setup(&pool).await;
    let router = app(AppState { pool: pool.clone() });

    post_movement(&router, &f, "receipt", "2027-04-01", "100").await;
    post_movement(&router, &f, "issue", "2027-04-10", "30").await;
    post_movement(&router, &f, "sales_return", "2027-04-15", "5").await;

    let body = on_hand(&router, &f, "2027-04-20").await;
    assert_eq!(body["onHand"], "75");
    assert_eq!(body["isLowStock"], false); // min_stock is 50, 75 > 50
    Ok(())
}

/// Manual test #2: on-hand as of a date before the last movement uses the date-windowed cumulative
/// sum, not the whole fiscal year.
#[sqlx::test(migrations = "./migrations")]
async fn on_hand_as_of_earlier_date_excludes_later_movements(pool: PgPool) -> sqlx::Result<()> {
    let f = setup(&pool).await;
    let router = app(AppState { pool: pool.clone() });

    post_movement(&router, &f, "receipt", "2027-04-01", "100").await;
    post_movement(&router, &f, "issue", "2027-04-10", "30").await; // after the query date below

    let body = on_hand(&router, &f, "2027-04-05").await;
    assert_eq!(body["onHand"], "100"); // the 30-unit sale on 04-10 must not count yet
    Ok(())
}

/// Manual test #3: a document's own lines are excluded from on-hand while it's being edited, by
/// real id, not the legacy's empty-string accident.
#[sqlx::test(migrations = "./migrations")]
async fn exclude_document_id_excludes_its_own_lines(pool: PgPool) -> sqlx::Result<()> {
    let f = setup(&pool).await;
    let router = app(AppState { pool: pool.clone() });

    post_movement(&router, &f, "receipt", "2027-04-01", "100").await;
    let editing_doc = post_movement(&router, &f, "issue", "2027-04-10", "30").await;

    let resp = req(
        &router, "GET",
        &format!(
            "/api/v1/items/{}/on-hand?fiscalYearId={}&asOfDate=2027-04-20&excludeDocumentId={editing_doc}",
            f.item_id, f.fiscal_year_id
        ),
        &f.token, Value::Null,
    )
    .await;
    let body = json_body(resp).await;
    assert_eq!(body["onHand"], "100"); // the 30-unit sale being edited must not count against itself
    Ok(())
}

/// Manual test #4: stock card running balance matches the on-hand query at every point in time,
/// and the opening balance is explicit (not zero) when the window starts mid-year.
#[sqlx::test(migrations = "./migrations")]
async fn stock_card_running_balance_matches_on_hand(pool: PgPool) -> sqlx::Result<()> {
    let f = setup(&pool).await;
    let router = app(AppState { pool: pool.clone() });

    post_movement(&router, &f, "receipt", "2027-04-01", "100").await;
    post_movement(&router, &f, "issue", "2027-04-10", "30").await;
    post_movement(&router, &f, "sales_return", "2027-04-15", "5").await;

    // Window starts after the first receipt -- opening balance must reflect it, not be zero.
    let resp = req(
        &router, "GET",
        &format!(
            "/api/v1/items/{}/stock-card?fiscalYearId={}&fromDate=2027-04-05&toDate=2027-04-20",
            f.item_id, f.fiscal_year_id
        ),
        &f.token, Value::Null,
    )
    .await;
    assert_eq!(resp.status(), StatusCode::OK);
    let card = json_body(resp).await;
    assert_eq!(card["openingBalance"], "100"); // the 04-01 receipt, before the window starts

    let movements = card["movements"].as_array().unwrap();
    assert_eq!(movements.len(), 2); // the sale and the sales return fall inside the window
    assert_eq!(movements[0]["runningBalance"], "70"); // 100 - 30
    assert_eq!(movements[1]["runningBalance"], "75"); // 70 + 5

    // The final running balance must equal on-hand queried independently at the same date.
    let body = on_hand(&router, &f, "2027-04-20").await;
    assert_eq!(movements.last().unwrap()["runningBalance"], body["onHand"]);
    Ok(())
}

/// Closes 5.1's deferred manual test #4: bringing on-hand below min_stock produces a real
/// `isLowStock` alert, not just a passive number next to the balance.
#[sqlx::test(migrations = "./migrations")]
async fn low_stock_alert_fires_when_on_hand_drops_below_threshold(pool: PgPool) -> sqlx::Result<()> {
    let f = setup(&pool).await; // min_stock = 50
    let router = app(AppState { pool: pool.clone() });

    post_movement(&router, &f, "receipt", "2027-04-01", "60").await;
    let body = on_hand(&router, &f, "2027-04-02").await;
    assert_eq!(body["onHand"], "60");
    assert_eq!(body["isLowStock"], false);

    post_movement(&router, &f, "issue", "2027-04-05", "20").await; // 60 - 20 = 40, below 50
    let body = on_hand(&router, &f, "2027-04-06").await;
    assert_eq!(body["onHand"], "40");
    assert_eq!(body["isLowStock"], true);
    Ok(())
}
