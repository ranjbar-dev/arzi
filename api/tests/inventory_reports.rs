//! Automated version of step 6.4's manual test (docs/phase-6-reporting.md
//! §6.4): B3 (no write), B16 (no runtime table), B25 (fiscal-year scoping
//! actually enforced), and stock-balance figures matching 5.3's canonical
//! on-hand formula exactly.

use api::{app, AppState};
use axum::{
    body::Body,
    http::{header, Request, StatusCode},
};
use serde_json::{json, Value};
use sqlx::PgPool;
use tower::ServiceExt;

#[derive(Clone)]
struct Fixture {
    tenant_id: i64,
    fiscal_year_id: i64,
    token: String,
    warehouse_id: i64,
    item_id: i64,
    counterparty_id: i64,
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

    // Six leaf accounts a warehouse needs (5.1's own posting-account set).
    let acc = |gl: i32| async move {
        sqlx::query_scalar::<_, i64>(
            "INSERT INTO accounts (tenant_id, general_ledger_code, name) VALUES ($1, $2, 'x') RETURNING id",
        )
        .bind(tenant_id)
        .bind(gl)
        .fetch_one(pool)
        .await
        .unwrap()
    };
    let purchase_acc = acc(501).await;
    let purchase_return_acc = acc(502).await;
    let sales_acc = acc(503).await;
    let sales_return_acc = acc(504).await;
    let discount_acc = acc(505).await;
    let vat_acc = acc(506).await;
    let counterparty_acc = acc(507).await;

    let warehouse_id: i64 = sqlx::query_scalar(
        "INSERT INTO warehouses \
         (tenant_id, name, purchase_account_id, purchase_return_account_id, sales_account_id, \
          sales_return_account_id, discount_account_id, vat_account_id) \
         VALUES ($1, 'Main', $2, $3, $4, $5, $6, $7) RETURNING id",
    )
    .bind(tenant_id)
    .bind(purchase_acc)
    .bind(purchase_return_acc)
    .bind(sales_acc)
    .bind(sales_return_acc)
    .bind(discount_acc)
    .bind(vat_acc)
    .fetch_one(pool)
    .await
    .unwrap();

    let uom_id: i64 = sqlx::query_scalar(
        "INSERT INTO units_of_measure (tenant_id, name) VALUES ($1, 'kg') RETURNING id",
    )
    .bind(tenant_id)
    .fetch_one(pool)
    .await
    .unwrap();
    let item_id: i64 = sqlx::query_scalar(
        "INSERT INTO items (tenant_id, code, name, unit_of_measure_id, sale_price, min_stock) \
         VALUES ($1, 5001, 'Pistachio', $2, 100000, 50) RETURNING id",
    )
    .bind(tenant_id)
    .bind(uom_id)
    .fetch_one(pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO item_warehouses (tenant_id, item_id, warehouse_id) VALUES ($1, $2, $3)",
    )
    .bind(tenant_id)
    .bind(item_id)
    .bind(warehouse_id)
    .execute(pool)
    .await
    .unwrap();

    Fixture {
        tenant_id,
        fiscal_year_id,
        token,
        warehouse_id,
        item_id,
        counterparty_id: counterparty_acc,
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

/// Creates a draft `receipt` or `issue` document with one line for the
/// fixture's item, on `date`, quantity `qty`, unit price `price`.
async fn make_document(
    router: &axum::Router,
    token: &str,
    fx: &Fixture,
    document_type: &str,
    date: &str,
    qty: f64,
    price: i64,
) -> i64 {
    let create = router
        .clone()
        .oneshot(
            Request::post("/api/v1/inventory-documents")
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::COOKIE, cookie(token))
                .body(Body::from(
                    json!({
                        "fiscalYearId": fx.fiscal_year_id, "documentType": document_type,
                        "warehouseId": fx.warehouse_id, "documentDate": date,
                        "counterpartyAccountId": fx.counterparty_id, "description": "activity fixture"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(
        create.status(),
        StatusCode::CREATED,
        "document create failed: {:?}",
        json_body(create).await
    );
    let doc_id = json_body(create).await["id"].as_i64().unwrap();

    let line = router
        .clone()
        .oneshot(
            Request::post(format!("/api/v1/inventory-documents/{doc_id}/lines"))
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::COOKIE, cookie(token))
                .body(Body::from(
                    json!({ "itemId": fx.item_id, "quantity": qty, "unitPrice": price })
                        .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(
        line.status(),
        StatusCode::CREATED,
        "line create failed: {:?}",
        json_body(line).await
    );
    doc_id
}

/// Manual test #3/#4 in one flow: a date range spanning two fiscal years
/// only ever returns the requested year's data (B25), and the stock-balance
/// figure matches an independently-posted set of documents exactly.
#[sqlx::test(migrations = "./migrations")]
async fn fiscal_year_scoping_and_stock_balance_agree_with_canonical_formula(
    pool: PgPool,
) -> sqlx::Result<()> {
    let fx = seed(&pool).await;
    let router = app(AppState { pool: pool.clone() });

    make_document(
        &router,
        &fx.token,
        &fx,
        "receipt",
        "2018-04-01",
        100.0,
        1000,
    )
    .await;
    make_document(&router, &fx.token, &fx, "issue", "2018-04-05", 30.0, 1000).await;

    // A second fiscal year with its own activity -- must not leak in.
    let other_fy: i64 = sqlx::query_scalar(
        "INSERT INTO fiscal_years (tenant_id, year, start_date, end_date) \
         VALUES ($1, 1398, '2019-03-21', '2020-03-20') RETURNING id",
    )
    .bind(fx.tenant_id)
    .fetch_one(&pool)
    .await?;
    let other_fx = Fixture {
        fiscal_year_id: other_fy,
        ..fx.clone()
    };
    make_document(
        &router,
        &fx.token,
        &other_fx,
        "receipt",
        "2019-04-01",
        9_999.0,
        1000,
    )
    .await;

    let resp = router
        .clone()
        .oneshot(
            Request::get(format!(
                "/api/v1/reports/inventory-activity?fiscalYearId={}&fromDate=2000-01-01&toDate=2028-01-01&groupBy=item",
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
    assert_eq!(
        rows.len(),
        1,
        "only the fixture's one item, from the requested year only"
    );
    // 100 + 30 = 130 units moved total, never touching the other year's 9,999.
    assert_eq!(rows[0]["quantity"], "130");

    let resp = router
        .oneshot(
            Request::get(format!(
                "/api/v1/reports/stock-balance?fiscalYearId={}&asOfDate=2028-01-01",
                fx.fiscal_year_id
            ))
            .header(header::COOKIE, cookie(&fx.token))
            .body(Body::empty())
            .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let stock = json_body(resp).await;
    let row = stock["rows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["itemId"] == fx.item_id)
        .unwrap();
    // 100 receipt - 30 issue = 70 on hand, matching 5.3's canonical formula.
    assert_eq!(row["onHand"], "70");
    assert_eq!(row["isLowStock"], false); // 70 > min_stock 50

    Ok(())
}

/// B3/B16 direct tests: running the report (in every `groupBy` shape) never
/// mutates the underlying data and never leaves a runtime table behind.
#[sqlx::test(migrations = "./migrations")]
async fn report_never_writes_and_creates_no_runtime_table(pool: PgPool) -> sqlx::Result<()> {
    let fx = seed(&pool).await;
    let router = app(AppState { pool: pool.clone() });
    make_document(
        &router,
        &fx.token,
        &fx,
        "receipt",
        "2018-04-01",
        100.0,
        1000,
    )
    .await;

    let before: (i64, i64) = sqlx::query_as(
        "SELECT count(*)::bigint, coalesce(sum(quantity), 0)::bigint FROM inventory_document_lines",
    )
    .fetch_one(&pool)
    .await?;

    for group_by in ["none", "date", "item", "date_item"] {
        let resp = router
            .clone()
            .oneshot(
                Request::get(format!(
                    "/api/v1/reports/inventory-activity?fiscalYearId={}&fromDate=2000-01-01&toDate=2028-01-01&groupBy={group_by}",
                    fx.fiscal_year_id
                ))
                .header(header::COOKIE, cookie(&fx.token))
                .body(Body::empty())
                .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(
            resp.status(),
            StatusCode::OK,
            "groupBy={group_by} failed: {:?}",
            json_body(resp).await
        );
    }

    let after: (i64, i64) = sqlx::query_as(
        "SELECT count(*)::bigint, coalesce(sum(quantity), 0)::bigint FROM inventory_document_lines",
    )
    .fetch_one(&pool)
    .await?;
    assert_eq!(
        before, after,
        "B3: running the report must never mutate inventory_document_lines"
    );

    // B16: no `temp_RJ_*`-style permanent table left behind anywhere.
    let leftover: i64 = sqlx::query_scalar(
        "SELECT count(*)::bigint FROM information_schema.tables \
         WHERE table_schema = 'public' AND table_name LIKE 'temp_%'",
    )
    .fetch_one(&pool)
    .await?;
    assert_eq!(leftover, 0);

    Ok(())
}
