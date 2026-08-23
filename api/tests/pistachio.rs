//! Automated version of step 5.6's manual test (docs/phase-5-inventory.md §5.6): Example A and
//! Example B worked exactly, required-field validation actually blocking a save, and the whole
//! flow reachable through the ordinary purchase-invoice endpoints (B19's core fix).

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
    /// A pistachio-grade item -- has `pistachio_grade_id` set, per §8.1.1's explicit-FK fix.
    pistachio_item_id: i64,
    /// An ordinary item with no grade, for the rejection case.
    ordinary_item_id: i64,
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

    req(&router, "POST", "/api/v1/pistachio-grades/seed-defaults", &token, Value::Null).await;
    let resp = req(&router, "GET", "/api/v1/pistachio-grades", &token, Value::Null).await;
    let grades = json_body(resp).await;
    let grade_id = grades.as_array().unwrap().iter().find(|g| g["name"] == "Ahmad-Aghaei").unwrap()["id"]
        .as_i64()
        .unwrap();

    let resp = req(
        &router, "POST", "/api/v1/items", &token,
        json!({
            "code": 5, "name": "Ahmad-Aghaei pistachio", "unitOfMeasureId": uom_id, "salePrice": 1,
            "pistachioGradeId": grade_id,
        }),
    )
    .await;
    let pistachio_item_id = json_body(resp).await["id"].as_i64().unwrap();

    let resp = req(
        &router, "POST", "/api/v1/items", &token,
        json!({ "code": 6, "name": "Ordinary item", "unitOfMeasureId": uom_id, "salePrice": 1 }),
    )
    .await;
    let ordinary_item_id = json_body(resp).await["id"].as_i64().unwrap();

    let resp = req(
        &router, "POST", "/api/v1/inventory-documents", &token,
        json!({
            "fiscalYearId": fiscal_year_id, "documentType": "receipt", "documentDate": "2027-05-01",
            "warehouseId": warehouse_id, "counterpartyAccountId": counterparty_id,
        }),
    )
    .await;
    let document_id = json_body(resp).await["id"].as_i64().unwrap();

    Fixture { token, document_id, pistachio_item_id, ordinary_item_id }
}

/// Manual test #1: Example A reproduced exactly through the real HTTP API — net_weight 1877.0 kg,
/// line_amount 2,346,250,000 rial.
#[sqlx::test(migrations = "./migrations")]
async fn example_a_reproduced_through_the_api(pool: PgPool) -> sqlx::Result<()> {
    let f = setup(&pool).await;
    let router = app(AppState { pool: pool.clone() });

    let body = json!({
        "itemId": f.pistachio_item_id,
        "baleCount": 40, "tareAllowanceKg": "0.2", "grossWeightKg": "2000.0",
        "moisturePct": "3.5", "blankPct": "2.0", "otherDeductionsKg": "5.0",
        "unitPrice": 1_250_000,
    });

    // Stateless preview first -- matches manual test #4's "reachable through the ordinary flow"
    // without persisting anything yet.
    let resp = req(&router, "POST", "/api/v1/pistachio-deduction/calculate", &f.token, body.clone()).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let preview = json_body(resp).await;
    assert_eq!(preview["netWeightKg"], "1877.000");
    assert_eq!(preview["lineAmount"], 2_346_250_000i64);

    // Then the real create-line action.
    let resp = req(
        &router, "POST", &format!("/api/v1/inventory-documents/{}/lines/pistachio", f.document_id), &f.token, body,
    )
    .await;
    assert_eq!(resp.status(), StatusCode::CREATED);
    let created = json_body(resp).await;
    assert_eq!(created["netWeightKg"], "1877.000");
    assert_eq!(created["lineAmount"], 2_346_250_000i64);

    let resp = req(&router, "GET", &format!("/api/v1/inventory-documents/{}", f.document_id), &f.token, Value::Null).await;
    let document = json_body(resp).await;
    assert_eq!(document["totalAmount"], 2_346_250_000i64);
    let lines = document["lines"].as_array().unwrap();
    assert_eq!(lines.len(), 1);
    // net weight became the line's quantity (compared numerically -- the DB round-trip's exact
    // trailing-zero scale isn't the thing under test, the value is).
    assert_eq!(lines[0]["quantity"].as_str().unwrap().parse::<f64>().unwrap(), 1877.0);
    assert_eq!(lines[0]["discountAmount"], 0); // §7.5: pistachio discount/VAT always 0
    assert_eq!(lines[0]["taxAmount"], 0);
    Ok(())
}

/// Manual test #2: Example B, the deduction floor — total deductions (565 kg) exceed gross
/// (500 kg), net weight floors to 0, line amount 0, and the preview surfaces this before saving.
#[sqlx::test(migrations = "./migrations")]
async fn example_b_deduction_floor(pool: PgPool) -> sqlx::Result<()> {
    let f = setup(&pool).await;
    let router = app(AppState { pool: pool.clone() });

    let body = json!({
        "itemId": f.pistachio_item_id,
        "baleCount": 40, "tareAllowanceKg": "1.0", "grossWeightKg": "500.0",
        "moisturePct": "60", "blankPct": "45", "otherDeductionsKg": "0",
        "unitPrice": 1_000_000,
    });
    let resp = req(&router, "POST", "/api/v1/pistachio-deduction/calculate", &f.token, body.clone()).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let preview = json_body(resp).await;
    assert_eq!(preview["totalDeductionKg"], "565.000"); // shown as-is, exceeding gross
    assert_eq!(preview["netWeightKg"], "0"); // floored
    assert_eq!(preview["lineAmount"], 0);

    // Saving a zero-amount pistachio line is still permitted (not silently blocked) -- it just
    // legitimately values at zero, matching the legacy's own "no error, no warning, no block".
    let resp = req(
        &router, "POST", &format!("/api/v1/inventory-documents/{}/lines/pistachio", f.document_id), &f.token, body,
    )
    .await;
    assert_eq!(resp.status(), StatusCode::CREATED);
    Ok(())
}

/// Manual test #3: bale count, gross weight and unit price missing/zero are rejected -- the real
/// validation fix, unlike the legacy's cosmetic red labels.
#[sqlx::test(migrations = "./migrations")]
async fn mandatory_fields_rejected_when_zero(pool: PgPool) -> sqlx::Result<()> {
    let f = setup(&pool).await;
    let router = app(AppState { pool: pool.clone() });

    let base = json!({
        "itemId": f.pistachio_item_id, "tareAllowanceKg": "0.2", "grossWeightKg": "100.0", "unitPrice": 1000,
    });

    let mut zero_bales = base.clone();
    zero_bales["baleCount"] = json!(0);
    let resp = req(&router, "POST", "/api/v1/pistachio-deduction/calculate", &f.token, zero_bales).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

    let mut missing_bales = base.clone();
    missing_bales.as_object_mut().unwrap().remove("baleCount");
    let resp = req(&router, "POST", "/api/v1/pistachio-deduction/calculate", &f.token, missing_bales).await;
    assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY); // required JSON field entirely absent

    let mut zero_gross = base.clone();
    zero_gross["baleCount"] = json!(40);
    zero_gross["grossWeightKg"] = json!("0");
    let resp = req(&router, "POST", "/api/v1/pistachio-deduction/calculate", &f.token, zero_gross).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

    let mut zero_price = base.clone();
    zero_price["baleCount"] = json!(40);
    zero_price["unitPrice"] = json!(0);
    let resp = req(&router, "POST", "/api/v1/pistachio-deduction/calculate", &f.token, zero_price).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    Ok(())
}

/// The item must actually be pistachio-grade (explicit FK, §8.1.1's fix) -- an ordinary item is
/// rejected rather than silently accepted through this endpoint.
#[sqlx::test(migrations = "./migrations")]
async fn non_pistachio_item_rejected(pool: PgPool) -> sqlx::Result<()> {
    let f = setup(&pool).await;
    let router = app(AppState { pool: pool.clone() });

    let body = json!({
        "itemId": f.ordinary_item_id,
        "baleCount": 1, "tareAllowanceKg": "0.2", "grossWeightKg": "10.0", "unitPrice": 1000,
    });
    let resp = req(
        &router, "POST", &format!("/api/v1/inventory-documents/{}/lines/pistachio", f.document_id), &f.token, body,
    )
    .await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    Ok(())
}
