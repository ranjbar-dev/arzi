//! Automated version of step 5.1's manual test (docs/phase-5-inventory.md §5.1): warehouses with
//! their six posting accounts, a unit of measure with a conversion factor, an item genuinely
//! assigned to two warehouses via the real junction table (not the legacy's single-scalar/CSV
//! designs), and item-code duplicate/immutability behaviour.

use api::{app, AppState};
use axum::{
    body::Body,
    http::{header, Request, StatusCode},
};
use serde_json::{json, Value};
use sqlx::PgPool;
use tower::ServiceExt;

async fn seed_session(pool: &PgPool) -> (i64, i64, String) {
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
    (tenant_id, user_id, token)
}

fn cookie(token: &str) -> String {
    format!("arzi_session={token}")
}

async fn json_body(response: axum::response::Response) -> Value {
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

async fn seed_leaf_account(pool: &PgPool, tenant_id: i64, gl: i32, sub: i32, a1: i32, a2: i32, name: &str) -> i64 {
    sqlx::query_scalar(
        "INSERT INTO accounts (tenant_id, general_ledger_code, subsidiary_code, analytic1_code, \
         analytic2_code, name) VALUES ($1, $2, $3, $4, $5, $6) RETURNING id",
    )
    .bind(tenant_id)
    .bind(gl)
    .bind(sub)
    .bind(a1)
    .bind(a2)
    .bind(name)
    .fetch_one(pool)
    .await
    .unwrap()
}

fn warehouse_payload(name: &str, accounts: [i64; 6]) -> Value {
    json!({
        "name": name,
        "vatRatePct": "9.00",
        "purchaseAccountId": accounts[0],
        "purchaseReturnAccountId": accounts[1],
        "salesAccountId": accounts[2],
        "salesReturnAccountId": accounts[3],
        "discountAccountId": accounts[4],
        "vatAccountId": accounts[5],
    })
}

async fn create_warehouse(router: &axum::Router, token: &str, name: &str, accounts: [i64; 6]) -> i64 {
    let resp = router
        .clone()
        .oneshot(
            Request::post("/api/v1/warehouses")
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::COOKIE, cookie(token))
                .body(Body::from(warehouse_payload(name, accounts).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    json_body(resp).await["id"].as_i64().unwrap()
}

async fn create_unit(router: &axum::Router, token: &str, body: Value) -> i64 {
    let resp = router
        .clone()
        .oneshot(
            Request::post("/api/v1/units-of-measure")
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::COOKIE, cookie(token))
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    json_body(resp).await["id"].as_i64().unwrap()
}

/// Manual test #1: two warehouses, each with its own six posting accounts and VAT rate.
#[sqlx::test(migrations = "./migrations")]
async fn create_warehouse_with_six_posting_accounts(pool: PgPool) -> sqlx::Result<()> {
    let (tenant_id, _uid, token) = seed_session(&pool).await;
    let router = app(AppState { pool: pool.clone() });

    let accounts = [
        seed_leaf_account(&pool, tenant_id, 500, 1, 0, 0, "Purchases").await,
        seed_leaf_account(&pool, tenant_id, 500, 2, 0, 0, "Purchase returns").await,
        seed_leaf_account(&pool, tenant_id, 400, 1, 0, 0, "Sales").await,
        seed_leaf_account(&pool, tenant_id, 400, 2, 0, 0, "Sales returns").await,
        seed_leaf_account(&pool, tenant_id, 600, 1, 0, 0, "Discounts").await,
        seed_leaf_account(&pool, tenant_id, 600, 2, 0, 0, "VAT").await,
    ];

    let id1 = create_warehouse(&router, &token, "Main warehouse", accounts).await;
    let id2 = create_warehouse(&router, &token, "Secondary warehouse", accounts).await;
    assert_ne!(id1, id2);

    let resp = router
        .clone()
        .oneshot(
            Request::get(format!("/api/v1/warehouses/{id1}"))
                .header(header::COOKIE, cookie(&token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = json_body(resp).await;
    assert_eq!(body["name"], "Main warehouse");
    assert_eq!(body["vatRatePct"], "9"); // BigDecimal serde strips trailing zeros
    assert_eq!(body["purchaseAccountId"].as_i64().unwrap(), accounts[0]);

    // A non-leaf posting account is rejected (5.8's engine requires leaf-only postings).
    let kol_only = seed_leaf_account(&pool, tenant_id, 700, 0, 0, 0, "Non-leaf Kol").await;
    sqlx::query("UPDATE accounts SET child_count = 1 WHERE id = $1")
        .bind(kol_only)
        .execute(&pool)
        .await
        .unwrap();
    let mut bad_accounts = accounts;
    bad_accounts[0] = kol_only;
    let resp = router
        .clone()
        .oneshot(
            Request::post("/api/v1/warehouses")
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::COOKIE, cookie(&token))
                .body(Body::from(warehouse_payload("Bad warehouse", bad_accounts).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    Ok(())
}

/// Manual test #2: a unit of measure with a conversion factor, usable via the API (not a
/// direct-SQL-only table, unlike the legacy).
#[sqlx::test(migrations = "./migrations")]
async fn unit_of_measure_with_conversion_factor(pool: PgPool) -> sqlx::Result<()> {
    let (_tenant_id, _uid, token) = seed_session(&pool).await;
    let router = app(AppState { pool: pool.clone() });

    let kg_id = create_unit(&router, &token, json!({ "name": "kg" })).await;
    let tonne_id = create_unit(
        &router,
        &token,
        json!({ "name": "tonne", "baseUnitId": kg_id, "conversionFactor": "1000" }),
    )
    .await;

    let resp = router
        .clone()
        .oneshot(
            Request::get("/api/v1/units-of-measure")
                .header(header::COOKIE, cookie(&token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let body = json_body(resp).await;
    let tonne = body.as_array().unwrap().iter().find(|u| u["id"] == tonne_id).unwrap();
    assert_eq!(tonne["baseUnitId"].as_i64().unwrap(), kg_id);
    assert_eq!(tonne["conversionFactor"], "1000");

    // A unit cannot be its own base, and cannot chain onto an already-derived unit.
    let resp = router
        .clone()
        .oneshot(
            Request::post("/api/v1/units-of-measure")
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::COOKIE, cookie(&token))
                .body(Body::from(
                    json!({ "name": "gram", "baseUnitId": tonne_id, "conversionFactor": "0.001" }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    Ok(())
}

/// Manual test #3: create an item, assign it to both warehouses via the junction table, confirm
/// it's queryable from either warehouse context — unlike the legacy's single-scalar `AJ_ID`.
#[sqlx::test(migrations = "./migrations")]
async fn item_assigned_to_two_warehouses_via_real_junction(pool: PgPool) -> sqlx::Result<()> {
    let (tenant_id, _uid, token) = seed_session(&pool).await;
    let router = app(AppState { pool: pool.clone() });

    let accounts = [
        seed_leaf_account(&pool, tenant_id, 500, 1, 0, 0, "Purchases").await,
        seed_leaf_account(&pool, tenant_id, 500, 2, 0, 0, "Purchase returns").await,
        seed_leaf_account(&pool, tenant_id, 400, 1, 0, 0, "Sales").await,
        seed_leaf_account(&pool, tenant_id, 400, 2, 0, 0, "Sales returns").await,
        seed_leaf_account(&pool, tenant_id, 600, 1, 0, 0, "Discounts").await,
        seed_leaf_account(&pool, tenant_id, 600, 2, 0, 0, "VAT").await,
    ];
    let wh1 = create_warehouse(&router, &token, "WH1", accounts).await;
    let wh2 = create_warehouse(&router, &token, "WH2", accounts).await;
    let kg_id = create_unit(&router, &token, json!({ "name": "kg" })).await;

    let resp = router
        .clone()
        .oneshot(
            Request::post("/api/v1/items")
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::COOKIE, cookie(&token))
                .body(Body::from(
                    json!({
                        "code": 1001,
                        "name": "Pistachio 500g bag",
                        "unitOfMeasureId": kg_id,
                        "salePrice": 250000,
                        "warehouseIds": [wh1, wh2],
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    let item_id = json_body(resp).await["id"].as_i64().unwrap();

    // Queryable from either warehouse context.
    for wh in [wh1, wh2] {
        let resp = router
            .clone()
            .oneshot(
                Request::get(format!("/api/v1/items?warehouseId={wh}"))
                    .header(header::COOKIE, cookie(&token))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let body = json_body(resp).await;
        assert!(body.as_array().unwrap().iter().any(|i| i["id"] == item_id), "missing from warehouse {wh}");
    }

    // Duplicate code rejected.
    let resp = router
        .clone()
        .oneshot(
            Request::post("/api/v1/items")
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::COOKIE, cookie(&token))
                .body(Body::from(
                    json!({ "code": 1001, "name": "Dup", "unitOfMeasureId": kg_id, "salePrice": 1 }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CONFLICT);

    // Zero sale price rejected (§2.2 check 4).
    let resp = router
        .clone()
        .oneshot(
            Request::post("/api/v1/items")
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::COOKIE, cookie(&token))
                .body(Body::from(
                    json!({ "code": 1002, "name": "Zero price", "unitOfMeasureId": kg_id, "salePrice": 0 })
                        .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

    // Unassign from one warehouse — item still exists but drops out of that warehouse's list.
    let resp = router
        .clone()
        .oneshot(
            Request::delete(format!("/api/v1/items/{item_id}/warehouses/{wh1}"))
                .header(header::COOKIE, cookie(&token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);
    let resp = router
        .clone()
        .oneshot(
            Request::get(format!("/api/v1/items?warehouseId={wh1}"))
                .header(header::COOKIE, cookie(&token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let body = json_body(resp).await;
    assert!(!body.as_array().unwrap().iter().any(|i| i["id"] == item_id));

    Ok(())
}

/// Item code is immutable after creation, matching §2.1/§2.3 exactly — but unlike the legacy,
/// editing is always by the real surrogate id, so there is no "code 0 is uneditable" hazard.
#[sqlx::test(migrations = "./migrations")]
async fn item_code_is_immutable_and_update_never_touches_it(pool: PgPool) -> sqlx::Result<()> {
    let (_tenant_id, _uid, token) = seed_session(&pool).await;
    let router = app(AppState { pool: pool.clone() });
    let kg_id = create_unit(&router, &token, json!({ "name": "kg" })).await;

    let resp = router
        .clone()
        .oneshot(
            Request::post("/api/v1/items")
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::COOKIE, cookie(&token))
                .body(Body::from(
                    json!({ "code": 5, "name": "Widget", "unitOfMeasureId": kg_id, "salePrice": 100 }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    let item_id = json_body(resp).await["id"].as_i64().unwrap();

    let resp = router
        .clone()
        .oneshot(
            Request::put(format!("/api/v1/items/{item_id}"))
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::COOKIE, cookie(&token))
                .body(Body::from(
                    json!({ "name": "Widget renamed", "unitOfMeasureId": kg_id, "salePrice": 200 }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    let resp = router
        .clone()
        .oneshot(
            Request::get(format!("/api/v1/items/{item_id}"))
                .header(header::COOKIE, cookie(&token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let body = json_body(resp).await;
    assert_eq!(body["code"], 5);
    assert_eq!(body["name"], "Widget renamed");
    assert_eq!(body["salePrice"], 200);

    // Deactivate is the real retirement path the legacy never had.
    let resp = router
        .clone()
        .oneshot(
            Request::post(format!("/api/v1/items/{item_id}/deactivate"))
                .header(header::COOKIE, cookie(&token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);
    let resp = router
        .clone()
        .oneshot(
            Request::get("/api/v1/items?activeOnly=true")
                .header(header::COOKIE, cookie(&token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let body = json_body(resp).await;
    assert!(!body.as_array().unwrap().iter().any(|i| i["id"] == item_id));

    Ok(())
}

/// Warehouse deactivation is real (unlike the legacy's dead "N2" delete handler) but only once
/// empty of item assignments.
#[sqlx::test(migrations = "./migrations")]
async fn warehouse_deactivate_requires_empty(pool: PgPool) -> sqlx::Result<()> {
    let (tenant_id, _uid, token) = seed_session(&pool).await;
    let router = app(AppState { pool: pool.clone() });

    let accounts = [
        seed_leaf_account(&pool, tenant_id, 500, 1, 0, 0, "Purchases").await,
        seed_leaf_account(&pool, tenant_id, 500, 2, 0, 0, "Purchase returns").await,
        seed_leaf_account(&pool, tenant_id, 400, 1, 0, 0, "Sales").await,
        seed_leaf_account(&pool, tenant_id, 400, 2, 0, 0, "Sales returns").await,
        seed_leaf_account(&pool, tenant_id, 600, 1, 0, 0, "Discounts").await,
        seed_leaf_account(&pool, tenant_id, 600, 2, 0, 0, "VAT").await,
    ];
    let wh1 = create_warehouse(&router, &token, "WH1", accounts).await;
    let kg_id = create_unit(&router, &token, json!({ "name": "kg" })).await;

    let resp = router
        .clone()
        .oneshot(
            Request::post("/api/v1/items")
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::COOKIE, cookie(&token))
                .body(Body::from(
                    json!({
                        "code": 1, "name": "Item", "unitOfMeasureId": kg_id, "salePrice": 10,
                        "warehouseIds": [wh1],
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    let item_id = json_body(resp).await["id"].as_i64().unwrap();

    let resp = router
        .clone()
        .oneshot(
            Request::post(format!("/api/v1/warehouses/{wh1}/deactivate"))
                .header(header::COOKIE, cookie(&token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

    router
        .clone()
        .oneshot(
            Request::delete(format!("/api/v1/items/{item_id}/warehouses/{wh1}"))
                .header(header::COOKIE, cookie(&token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let resp = router
        .clone()
        .oneshot(
            Request::post(format!("/api/v1/warehouses/{wh1}/deactivate"))
                .header(header::COOKIE, cookie(&token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);
    Ok(())
}

/// Manual test's pistachio-grade seeding: idempotent, tenant-scoped, superuser-only.
#[sqlx::test(migrations = "./migrations")]
async fn pistachio_grades_seed_defaults_is_idempotent(pool: PgPool) -> sqlx::Result<()> {
    let (_tenant_id, _uid, token) = seed_session(&pool).await;
    let router = app(AppState { pool: pool.clone() });

    for _ in 0..2 {
        let resp = router
            .clone()
            .oneshot(
                Request::post("/api/v1/pistachio-grades/seed-defaults")
                    .header(header::COOKIE, cookie(&token))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::NO_CONTENT);
    }

    let resp = router
        .clone()
        .oneshot(
            Request::get("/api/v1/pistachio-grades")
                .header(header::COOKIE, cookie(&token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let body = json_body(resp).await;
    assert_eq!(body.as_array().unwrap().len(), 7);
    assert_eq!(body[4]["name"], "Ahmad-Aghaei");
    Ok(())
}
