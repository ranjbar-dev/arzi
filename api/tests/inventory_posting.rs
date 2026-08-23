//! Automated version of step 5.8's manual test (docs/phase-5-inventory.md §5.8): a purchase with
//! discount+VAT posts perfectly balanced (the structural B1 fix), production and transfer post
//! real balanced vouchers (B2), un-posting removes every line (B8/B9), re-posting is idempotent
//! (no duplicates), and narration is accurate per type.

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
    tenant_id: i64,
    token: String,
    fiscal_year_id: i64,
    warehouse_id: i64,
    warehouse2_id: i64,
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
    let finished_goods = seed_leaf_account(pool, tenant_id, 903, 1, "Finished goods").await;
    let raw_materials = seed_leaf_account(pool, tenant_id, 903, 2, "Raw materials").await;
    let inventory1 = seed_leaf_account(pool, tenant_id, 904, 1, "Inventory WH1").await;
    let inventory2 = seed_leaf_account(pool, tenant_id, 904, 2, "Inventory WH2").await;

    let warehouse_id: i64 =
        {
            let resp = req(
            &router, "POST", "/api/v1/warehouses", &token,
            json!({
                "name": "Main", "vatRatePct": "9",
                "purchaseAccountId": accounts[0], "purchaseReturnAccountId": accounts[1],
                "salesAccountId": accounts[2], "salesReturnAccountId": accounts[3],
                "discountAccountId": accounts[4], "vatAccountId": accounts[5],
                "finishedGoodsAccountId": finished_goods, "rawMaterialsAccountId": raw_materials,
                "inventoryAccountId": inventory1,
            }),
        )
        .await;
            assert_eq!(resp.status(), StatusCode::CREATED);
            json_body(resp).await["id"].as_i64().unwrap()
        };
    let warehouse2_id: i64 =
        {
            let resp = req(
            &router, "POST", "/api/v1/warehouses", &token,
            json!({
                "name": "Secondary", "vatRatePct": "9",
                "purchaseAccountId": accounts[0], "purchaseReturnAccountId": accounts[1],
                "salesAccountId": accounts[2], "salesReturnAccountId": accounts[3],
                "discountAccountId": accounts[4], "vatAccountId": accounts[5],
                "finishedGoodsAccountId": finished_goods, "rawMaterialsAccountId": raw_materials,
                "inventoryAccountId": inventory2,
            }),
        )
        .await;
            json_body(resp).await["id"].as_i64().unwrap()
        };

    let counterparty_id = seed_leaf_account(pool, tenant_id, 103, 1, "Trade AR").await;
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

    Fixture {
        tenant_id,
        token,
        fiscal_year_id,
        warehouse_id,
        warehouse2_id,
        counterparty_id,
        item_id,
    }
}

async fn voucher_totals(pool: &PgPool, voucher_id: i64) -> (i64, i64, i32) {
    sqlx::query_as("SELECT total_debit, total_credit, line_count FROM vouchers WHERE id = $1")
        .bind(voucher_id)
        .fetch_one(pool)
        .await
        .unwrap()
}

/// Manual test #1: a purchase with a discount and VAT posts a perfectly balanced voucher — the
/// exact case (2·discount − VAT) that would have imbalanced under the legacy's `init12`.
#[sqlx::test(migrations = "./migrations")]
async fn purchase_with_discount_and_vat_posts_balanced(pool: PgPool) -> sqlx::Result<()> {
    let f = setup(&pool).await;
    let router = app(AppState { pool: pool.clone() });

    let resp = req(
        &router, "POST", "/api/v1/inventory-documents", &f.token,
        json!({
            "fiscalYearId": f.fiscal_year_id, "documentType": "receipt", "documentDate": "2027-05-01",
            "warehouseId": f.warehouse_id, "counterpartyAccountId": f.counterparty_id,
        }),
    )
    .await;
    let document_id = json_body(resp).await["id"].as_i64().unwrap();

    // gross 10,000,000; discount 500,000; VAT computed on (gross - discount) at 9% = 855,000.
    let resp = req(
        &router, "POST", &format!("/api/v1/inventory-documents/{document_id}/lines"), &f.token,
        json!({ "itemId": f.item_id, "quantity": "10", "unitPrice": 1_000_000, "discountAmount": 500_000, "taxAmount": 855_000 }),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::CREATED);

    let resp = req(
        &router,
        "POST",
        &format!("/api/v1/inventory-documents/{document_id}/post"),
        &f.token,
        Value::Null,
    )
    .await;
    assert_eq!(resp.status(), StatusCode::NO_CONTENT, "post failed");

    let resp = req(
        &router,
        "GET",
        &format!("/api/v1/inventory-documents/{document_id}"),
        &f.token,
        Value::Null,
    )
    .await;
    let body = json_body(resp).await;
    let voucher_id = body["postedVoucherId"].as_i64().unwrap();
    let (debit, credit, _lines) = voucher_totals(&pool, voucher_id).await;
    assert_eq!(debit, credit, "voucher must balance");
    assert_eq!(debit, 10_000_000 + 855_000); // gross + tax on both sides
    Ok(())
}

/// Manual test #2: production and transfer post real, balanced vouchers -- not silence.
#[sqlx::test(migrations = "./migrations")]
async fn production_and_transfer_post_balanced_vouchers(pool: PgPool) -> sqlx::Result<()> {
    let f = setup(&pool).await;
    let router = app(AppState { pool: pool.clone() });

    // Production.
    let resp = req(
        &router, "POST", "/api/v1/inventory-documents", &f.token,
        json!({
            "fiscalYearId": f.fiscal_year_id, "documentType": "production", "documentDate": "2027-05-01",
            "warehouseId": f.warehouse_id,
        }),
    )
    .await;
    assert_eq!(
        resp.status(),
        StatusCode::CREATED,
        "{:?}",
        json_body(resp).await
    );
    let production_id = json_body(resp).await["id"].as_i64().unwrap();
    req(
        &router,
        "POST",
        &format!("/api/v1/inventory-documents/{production_id}/lines"),
        &f.token,
        json!({ "itemId": f.item_id, "quantity": "5", "unitPrice": 200_000 }),
    )
    .await;
    let resp = req(
        &router,
        "POST",
        &format!("/api/v1/inventory-documents/{production_id}/post"),
        &f.token,
        Value::Null,
    )
    .await;
    assert_eq!(
        resp.status(),
        StatusCode::NO_CONTENT,
        "production post failed"
    );
    let resp = req(
        &router,
        "GET",
        &format!("/api/v1/inventory-documents/{production_id}"),
        &f.token,
        Value::Null,
    )
    .await;
    let voucher_id = json_body(resp).await["postedVoucherId"].as_i64().unwrap();
    let (debit, credit, lines) = voucher_totals(&pool, voucher_id).await;
    assert_eq!(debit, credit);
    assert_eq!(debit, 1_000_000);
    assert_eq!(lines, 2);

    // Transfer.
    let resp = req(
        &router, "POST", "/api/v1/inventory-documents", &f.token,
        json!({
            "fiscalYearId": f.fiscal_year_id, "documentType": "transfer", "documentDate": "2027-05-01",
            "warehouseId": f.warehouse_id, "destinationWarehouseId": f.warehouse2_id,
        }),
    )
    .await;
    assert_eq!(
        resp.status(),
        StatusCode::CREATED,
        "{:?}",
        json_body(resp).await
    );
    let transfer_id = json_body(resp).await["id"].as_i64().unwrap();
    req(
        &router,
        "POST",
        &format!("/api/v1/inventory-documents/{transfer_id}/lines"),
        &f.token,
        json!({ "itemId": f.item_id, "quantity": "3", "unitPrice": 100_000 }),
    )
    .await;
    let resp = req(
        &router,
        "POST",
        &format!("/api/v1/inventory-documents/{transfer_id}/post"),
        &f.token,
        Value::Null,
    )
    .await;
    assert_eq!(
        resp.status(),
        StatusCode::NO_CONTENT,
        "transfer post failed"
    );
    let resp = req(
        &router,
        "GET",
        &format!("/api/v1/inventory-documents/{transfer_id}"),
        &f.token,
        Value::Null,
    )
    .await;
    let voucher_id = json_body(resp).await["postedVoucherId"].as_i64().unwrap();
    let (debit, credit, lines) = voucher_totals(&pool, voucher_id).await;
    assert_eq!(debit, credit);
    assert_eq!(debit, 300_000);
    assert_eq!(lines, 2);
    Ok(())
}

/// Manual test #3: un-posting (deleting a posted document) removes every posting line.
#[sqlx::test(migrations = "./migrations")]
async fn deleting_a_posted_document_removes_the_whole_voucher(pool: PgPool) -> sqlx::Result<()> {
    let f = setup(&pool).await;
    let router = app(AppState { pool: pool.clone() });

    let resp = req(
        &router, "POST", "/api/v1/inventory-documents", &f.token,
        json!({
            "fiscalYearId": f.fiscal_year_id, "documentType": "receipt", "documentDate": "2027-05-01",
            "warehouseId": f.warehouse_id, "counterpartyAccountId": f.counterparty_id,
        }),
    )
    .await;
    let document_id = json_body(resp).await["id"].as_i64().unwrap();
    req(
        &router,
        "POST",
        &format!("/api/v1/inventory-documents/{document_id}/lines"),
        &f.token,
        json!({ "itemId": f.item_id, "quantity": "1", "unitPrice": 100_000 }),
    )
    .await;
    req(
        &router,
        "POST",
        &format!("/api/v1/inventory-documents/{document_id}/post"),
        &f.token,
        Value::Null,
    )
    .await;
    let resp = req(
        &router,
        "GET",
        &format!("/api/v1/inventory-documents/{document_id}"),
        &f.token,
        Value::Null,
    )
    .await;
    let voucher_id = json_body(resp).await["postedVoucherId"].as_i64().unwrap();

    let resp = req(
        &router,
        "DELETE",
        &format!("/api/v1/inventory-documents/{document_id}"),
        &f.token,
        Value::Null,
    )
    .await;
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    let voucher_exists: bool =
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM vouchers WHERE id = $1)")
            .bind(voucher_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert!(!voucher_exists, "voucher must be gone");
    let lines_left: i64 =
        sqlx::query_scalar("SELECT count(*) FROM voucher_lines WHERE voucher_id = $1")
            .bind(voucher_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(lines_left, 0, "no orphaned voucher lines");
    Ok(())
}

/// Manual test #4: re-posting an already-posted document (after editing a line) is idempotent --
/// exactly one voucher exists afterward, with the updated amount, never duplicated.
#[sqlx::test(migrations = "./migrations")]
async fn reposting_is_idempotent_no_duplicates(pool: PgPool) -> sqlx::Result<()> {
    let f = setup(&pool).await;
    let router = app(AppState { pool: pool.clone() });

    let resp = req(
        &router,
        "POST",
        "/api/v1/inventory-documents",
        &f.token,
        json!({
            "fiscalYearId": f.fiscal_year_id, "documentType": "issue", "documentDate": "2027-05-01",
            "warehouseId": f.warehouse_id, "counterpartyAccountId": f.counterparty_id,
        }),
    )
    .await;
    let document_id = json_body(resp).await["id"].as_i64().unwrap();
    let resp = req(
        &router,
        "POST",
        &format!("/api/v1/inventory-documents/{document_id}/lines"),
        &f.token,
        json!({ "itemId": f.item_id, "quantity": "1", "unitPrice": 100_000 }),
    )
    .await;
    let line_id = json_body(resp).await["id"].as_i64().unwrap();

    req(
        &router,
        "POST",
        &format!("/api/v1/inventory-documents/{document_id}/post"),
        &f.token,
        Value::Null,
    )
    .await;
    let resp = req(
        &router,
        "GET",
        &format!("/api/v1/inventory-documents/{document_id}"),
        &f.token,
        Value::Null,
    )
    .await;
    let first_voucher_id = json_body(resp).await["postedVoucherId"].as_i64().unwrap();

    // Edit the line while posted (5.8 relaxes the draft-only guard) then re-post.
    let resp = req(
        &router,
        "PUT",
        &format!("/api/v1/inventory-documents/{document_id}/lines/{line_id}"),
        &f.token,
        json!({ "itemId": f.item_id, "quantity": "2", "unitPrice": 100_000 }),
    )
    .await;
    assert_eq!(
        resp.status(),
        StatusCode::NO_CONTENT,
        "editing a posted line should now be allowed"
    );

    let resp = req(
        &router,
        "POST",
        &format!("/api/v1/inventory-documents/{document_id}/post"),
        &f.token,
        Value::Null,
    )
    .await;
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    let resp = req(
        &router,
        "GET",
        &format!("/api/v1/inventory-documents/{document_id}"),
        &f.token,
        Value::Null,
    )
    .await;
    let body = json_body(resp).await;
    let second_voucher_id = body["postedVoucherId"].as_i64().unwrap();
    assert_ne!(
        first_voucher_id, second_voucher_id,
        "re-post should replace the stale voucher"
    );

    let first_voucher_still_exists: bool =
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM vouchers WHERE id = $1)")
            .bind(first_voucher_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert!(
        !first_voucher_still_exists,
        "the stale voucher must be gone, not duplicated alongside the new one"
    );

    let voucher_count: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM vouchers WHERE tenant_id = $1 AND id IN (SELECT posted_voucher_id FROM inventory_documents WHERE id = $2)",
    )
    .bind(f.tenant_id)
    .bind(document_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(voucher_count, 1);

    let (debit, credit, _) = voucher_totals(&pool, second_voucher_id).await;
    assert_eq!(debit, 200_000); // reflects the updated quantity, not the stale amount
    assert_eq!(debit, credit);
    Ok(())
}

/// Manual test #5: each document type gets an accurate, distinct narration -- never the legacy's
/// shared "goods sale" label.
#[sqlx::test(migrations = "./migrations")]
async fn narration_is_accurate_and_distinct_per_type(pool: PgPool) -> sqlx::Result<()> {
    let f = setup(&pool).await;
    let router = app(AppState { pool: pool.clone() });

    let mut narrations = std::collections::HashSet::new();
    for document_type in ["receipt", "issue", "purchase_return", "sales_return"] {
        let resp = req(
            &router, "POST", "/api/v1/inventory-documents", &f.token,
            json!({
                "fiscalYearId": f.fiscal_year_id, "documentType": document_type, "documentDate": "2027-05-01",
                "warehouseId": f.warehouse_id, "counterpartyAccountId": f.counterparty_id,
            }),
        )
        .await;
        let document_id = json_body(resp).await["id"].as_i64().unwrap();
        req(
            &router,
            "POST",
            &format!("/api/v1/inventory-documents/{document_id}/lines"),
            &f.token,
            json!({ "itemId": f.item_id, "quantity": "1", "unitPrice": 10_000 }),
        )
        .await;
        req(
            &router,
            "POST",
            &format!("/api/v1/inventory-documents/{document_id}/post"),
            &f.token,
            Value::Null,
        )
        .await;
        let resp = req(
            &router,
            "GET",
            &format!("/api/v1/inventory-documents/{document_id}"),
            &f.token,
            Value::Null,
        )
        .await;
        let voucher_id = json_body(resp).await["postedVoucherId"].as_i64().unwrap();
        let description: String =
            sqlx::query_scalar("SELECT description FROM vouchers WHERE id = $1")
                .bind(voucher_id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert!(
            !description.to_lowercase().contains("goods sale"),
            "must not be the legacy's shared label"
        );
        assert!(
            narrations.insert(description),
            "narration must be distinct per document type"
        );
    }
    assert_eq!(narrations.len(), 4);
    Ok(())
}
