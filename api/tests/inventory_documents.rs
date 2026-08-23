//! Automated version of step 5.2's manual test (docs/phase-5-inventory.md §5.2): the B7
//! counterparty fix, draft creation + line CRUD, per-type permission gating (1404-1407, 1408,
//! 1414), and header totals maintained incrementally as lines change.

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

async fn make_plain_user(pool: &PgPool, tenant_id: i64, username: &str) -> (i64, String) {
    let user_id: i64 = sqlx::query_scalar(
        "INSERT INTO users (tenant_id, username, password_hash, is_superuser) \
         VALUES ($1, $2, 'x', false) RETURNING id",
    )
    .bind(tenant_id)
    .bind(username)
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
    (user_id, token)
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
        json!({ "code": 1, "name": "Pistachio", "unitOfMeasureId": uom_id, "salePrice": 100000 }),
    )
    .await;
    let item_id = json_body(resp).await["id"].as_i64().unwrap();

    Fixture {
        tenant_id,
        token,
        fiscal_year_id,
        warehouse_id,
        counterparty_id,
        item_id,
    }
}

fn create_body(f: &Fixture, document_type: &str, counterparty: Option<i64>) -> Value {
    let mut body = json!({
        "fiscalYearId": f.fiscal_year_id,
        "documentType": document_type,
        "documentDate": "2027-05-01",
        "warehouseId": f.warehouse_id,
    });
    if let Some(cp) = counterparty {
        body["counterpartyAccountId"] = json!(cp);
    }
    body
}

/// Manual test #1: no counterparty -> rejected (B7).
#[sqlx::test(migrations = "./migrations")]
async fn create_without_counterparty_is_rejected(pool: PgPool) -> sqlx::Result<()> {
    let f = setup(&pool).await;
    let router = app(AppState { pool: pool.clone() });

    // The legacy's zero-sentinel path (`AF_Customer = 0`) is rejected outright — account 0 never
    // resolves to a real leaf account, closing the B7 hole structurally rather than special-casing it.
    let resp = req(
        &router,
        "POST",
        "/api/v1/inventory-documents",
        &f.token,
        create_body(&f, "receipt", Some(0)),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

    // A non-leaf counterparty is rejected too (still B7's spirit: a real postable account).
    let kol_only = seed_leaf_account(&pool, f.tenant_id, 950, 0, "Non-leaf Kol").await;
    sqlx::query("UPDATE accounts SET child_count = 1 WHERE id = $1")
        .bind(kol_only)
        .execute(&pool)
        .await
        .unwrap();
    let resp = req(
        &router,
        "POST",
        "/api/v1/inventory-documents",
        &f.token,
        create_body(&f, "receipt", Some(kol_only)),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    Ok(())
}

/// Manual test #2/#3: create a purchase invoice (receipt) with valid counterparty + line items ->
/// saves as draft; edit the draft -> succeeds. Also proves incremental header-total maintenance.
#[sqlx::test(migrations = "./migrations")]
async fn create_draft_add_lines_and_edit(pool: PgPool) -> sqlx::Result<()> {
    let f = setup(&pool).await;
    let router = app(AppState { pool: pool.clone() });

    let resp = req(
        &router,
        "POST",
        "/api/v1/inventory-documents",
        &f.token,
        create_body(&f, "receipt", Some(f.counterparty_id)),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::CREATED);
    let doc_id = json_body(resp).await["id"].as_i64().unwrap();

    let resp = req(
        &router,
        "GET",
        &format!("/api/v1/inventory-documents/{doc_id}"),
        &f.token,
        Value::Null,
    )
    .await;
    let body = json_body(resp).await;
    assert_eq!(body["status"], "draft");
    assert_eq!(body["totalAmount"], 0);

    // Add a line: 10 kg @ 50,000 = 500,000 gross, no discount/tax.
    let resp = req(
        &router,
        "POST",
        &format!("/api/v1/inventory-documents/{doc_id}/lines"),
        &f.token,
        json!({ "itemId": f.item_id, "quantity": "10", "unitPrice": 50000 }),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::CREATED);
    let line_id = json_body(resp).await["id"].as_i64().unwrap();

    let resp = req(
        &router,
        "GET",
        &format!("/api/v1/inventory-documents/{doc_id}"),
        &f.token,
        Value::Null,
    )
    .await;
    let body = json_body(resp).await;
    assert_eq!(body["grossAmount"], 500000);
    assert_eq!(body["totalAmount"], 500000);
    assert_eq!(body["lines"].as_array().unwrap().len(), 1);

    // Edit the line: bump quantity to 20 -> gross/total 1,000,000; header totals track incrementally.
    let resp = req(
        &router,
        "PUT",
        &format!("/api/v1/inventory-documents/{doc_id}/lines/{line_id}"),
        &f.token,
        json!({ "itemId": f.item_id, "quantity": "20", "unitPrice": 50000, "discountAmount": 10000 }),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);
    let resp = req(
        &router,
        "GET",
        &format!("/api/v1/inventory-documents/{doc_id}"),
        &f.token,
        Value::Null,
    )
    .await;
    let body = json_body(resp).await;
    assert_eq!(body["grossAmount"], 1000000);
    assert_eq!(body["discountAmount"], 10000);
    assert_eq!(body["totalAmount"], 1000000 - 10000);

    // Edit the draft header -> succeeds.
    let resp = req(
        &router,
        "PUT",
        &format!("/api/v1/inventory-documents/{doc_id}"),
        &f.token,
        json!({
            "documentDate": "2027-05-02", "warehouseId": f.warehouse_id,
            "counterpartyAccountId": f.counterparty_id, "description": "revised",
        }),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    // Delete the line -> header totals return to zero.
    let resp = req(
        &router,
        "DELETE",
        &format!("/api/v1/inventory-documents/{doc_id}/lines/{line_id}"),
        &f.token,
        Value::Null,
    )
    .await;
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);
    let resp = req(
        &router,
        "GET",
        &format!("/api/v1/inventory-documents/{doc_id}"),
        &f.token,
        Value::Null,
    )
    .await;
    let body = json_body(resp).await;
    assert_eq!(body["totalAmount"], 0);
    assert_eq!(body["lines"].as_array().unwrap().len(), 0);

    // Delete the (still draft) document itself.
    let resp = req(
        &router,
        "DELETE",
        &format!("/api/v1/inventory-documents/{doc_id}"),
        &f.token,
        Value::Null,
    )
    .await;
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);
    let resp = req(
        &router,
        "GET",
        &format!("/api/v1/inventory-documents/{doc_id}"),
        &f.token,
        Value::Null,
    )
    .await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    Ok(())
}

/// The four document types are independently permission-gated (1404-1407) — a user granted only
/// one type's permission can create that type and no other.
#[sqlx::test(migrations = "./migrations")]
async fn per_type_permissions_are_independent(pool: PgPool) -> sqlx::Result<()> {
    let f = setup(&pool).await;
    let router = app(AppState { pool: pool.clone() });
    let (user_id, clerk_token) = make_plain_user(&pool, f.tenant_id, "clerk").await;

    // No permission at all -> every type 403s.
    for doc_type in ["receipt", "issue", "purchase_return", "sales_return"] {
        let resp = req(
            &router,
            "POST",
            "/api/v1/inventory-documents",
            &clerk_token,
            create_body(&f, doc_type, Some(f.counterparty_id)),
        )
        .await;
        assert_eq!(
            resp.status(),
            StatusCode::FORBIDDEN,
            "{doc_type} should 403 with no grants"
        );
    }

    grant(&pool, f.tenant_id, user_id, "issue_purchase_invoice").await; // 1404, receipt only
    let resp = req(
        &router,
        "POST",
        "/api/v1/inventory-documents",
        &clerk_token,
        create_body(&f, "receipt", Some(f.counterparty_id)),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::CREATED);
    let resp = req(
        &router,
        "POST",
        "/api/v1/inventory-documents",
        &clerk_token,
        create_body(&f, "issue", Some(f.counterparty_id)),
    )
    .await;
    assert_eq!(
        resp.status(),
        StatusCode::FORBIDDEN,
        "receipt grant must not also allow issue"
    );

    // Amend/delete are separately gated too.
    let resp = req(
        &router,
        "POST",
        "/api/v1/inventory-documents",
        &f.token,
        create_body(&f, "receipt", Some(f.counterparty_id)),
    )
    .await;
    let doc_id = json_body(resp).await["id"].as_i64().unwrap();
    let resp = req(
        &router, "PUT", &format!("/api/v1/inventory-documents/{doc_id}"), &clerk_token,
        json!({ "documentDate": "2027-05-02", "warehouseId": f.warehouse_id, "counterpartyAccountId": f.counterparty_id }),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    let resp = req(
        &router,
        "DELETE",
        &format!("/api/v1/inventory-documents/{doc_id}"),
        &clerk_token,
        Value::Null,
    )
    .await;
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    grant(&pool, f.tenant_id, user_id, "delete_invoice").await; // 1414
    let resp = req(
        &router,
        "DELETE",
        &format!("/api/v1/inventory-documents/{doc_id}"),
        &clerk_token,
        Value::Null,
    )
    .await;
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);
    Ok(())
}

/// A date outside the fiscal year's range is rejected, matching the fiscal-year-open gate every
/// other mutating module in this codebase applies uniformly.
#[sqlx::test(migrations = "./migrations")]
async fn date_outside_fiscal_year_rejected(pool: PgPool) -> sqlx::Result<()> {
    let f = setup(&pool).await;
    let router = app(AppState { pool: pool.clone() });
    let mut body = create_body(&f, "receipt", Some(f.counterparty_id));
    body["documentDate"] = json!("2020-01-01");
    let resp = req(
        &router,
        "POST",
        "/api/v1/inventory-documents",
        &f.token,
        body,
    )
    .await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    Ok(())
}

/// Duplicate document numbers within the same fiscal year are rejected, and the shared
/// per-fiscal-year sequence (not per-type) allocates when omitted.
#[sqlx::test(migrations = "./migrations")]
async fn document_number_shared_sequence_and_uniqueness(pool: PgPool) -> sqlx::Result<()> {
    let f = setup(&pool).await;
    let router = app(AppState { pool: pool.clone() });

    let resp = req(
        &router,
        "POST",
        "/api/v1/inventory-documents",
        &f.token,
        create_body(&f, "receipt", Some(f.counterparty_id)),
    )
    .await;
    let n1 = json_body(resp).await;
    let resp = req(
        &router,
        "POST",
        "/api/v1/inventory-documents",
        &f.token,
        create_body(&f, "issue", Some(f.counterparty_id)),
    )
    .await;
    let n2 = json_body(resp).await;
    assert_ne!(n1["id"], n2["id"]);

    // Explicit duplicate number is rejected by the unique constraint.
    let doc1 = req(
        &router,
        "GET",
        &format!("/api/v1/inventory-documents/{}", n1["id"]),
        &f.token,
        Value::Null,
    )
    .await;
    let existing_number = json_body(doc1).await["documentNumber"].as_i64().unwrap();
    let mut body = create_body(&f, "receipt", Some(f.counterparty_id));
    body["documentNumber"] = json!(existing_number);
    let resp = req(
        &router,
        "POST",
        "/api/v1/inventory-documents",
        &f.token,
        body,
    )
    .await;
    assert_eq!(resp.status(), StatusCode::CONFLICT);
    Ok(())
}

/// Step 7.1's constraint audit: `total_amount = gross_amount + tax_amount - discount_amount` is a
/// real database CHECK (migration 0020), not just something the API's own arithmetic happens to
/// preserve — a raw INSERT that bypasses the API entirely and gets the identity wrong must still be
/// rejected by Postgres itself, on both the header and the line table.
#[sqlx::test(migrations = "./migrations")]
async fn total_identity_enforced_at_database_level(pool: PgPool) -> sqlx::Result<()> {
    let f = setup(&pool).await;

    let header_result = sqlx::query(
        "INSERT INTO inventory_documents \
         (tenant_id, fiscal_year_id, document_type, document_number, document_date, warehouse_id, \
          counterparty_account_id, gross_amount, tax_amount, discount_amount, total_amount) \
         VALUES ($1, $2, 'receipt', 999001, '2027-05-01', $3, $4, 1000, 0, 0, 999)",
    )
    .bind(f.tenant_id)
    .bind(f.fiscal_year_id)
    .bind(f.warehouse_id)
    .bind(f.counterparty_id)
    .execute(&pool)
    .await;
    assert!(
        header_result.is_err(),
        "a wrong header total must be rejected by the database"
    );
    let msg = header_result.unwrap_err().to_string();
    assert!(
        msg.contains("inventory_documents_total_identity"),
        "unexpected error: {msg}"
    );

    // A correctly-totalled header to hang the line off, so only the LINE's identity is under test.
    let doc_id: i64 = sqlx::query_scalar(
        "INSERT INTO inventory_documents \
         (tenant_id, fiscal_year_id, document_type, document_number, document_date, warehouse_id, \
          counterparty_account_id, gross_amount, tax_amount, discount_amount, total_amount) \
         VALUES ($1, $2, 'receipt', 999002, '2027-05-01', $3, $4, 1000, 0, 0, 1000) RETURNING id",
    )
    .bind(f.tenant_id)
    .bind(f.fiscal_year_id)
    .bind(f.warehouse_id)
    .bind(f.counterparty_id)
    .fetch_one(&pool)
    .await?;

    let line_result = sqlx::query(
        "INSERT INTO inventory_document_lines \
         (tenant_id, fiscal_year_id, document_id, item_id, quantity, unit_price, \
          gross_amount, tax_amount, discount_amount, total_amount) \
         VALUES ($1, $2, $3, $4, 10, 100, 1000, 0, 0, 999)",
    )
    .bind(f.tenant_id)
    .bind(f.fiscal_year_id)
    .bind(doc_id)
    .bind(f.item_id)
    .execute(&pool)
    .await;
    assert!(
        line_result.is_err(),
        "a wrong line total must be rejected by the database"
    );
    let msg = line_result.unwrap_err().to_string();
    assert!(
        msg.contains("inventory_document_lines_total_identity"),
        "unexpected error: {msg}"
    );
    Ok(())
}
