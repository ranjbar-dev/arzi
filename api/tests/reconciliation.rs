//! Step 7.5 (docs/phase-7-hardening-and-cutover.md §7.5 / specs/10-target-architecture.md §6):
//! the reconciliation harness. `§6` calls this "the highest-value tests in this project" and asks
//! for it to exist even before a populated legacy database does, seeded from the worked-arithmetic
//! examples already embedded in the specs (party balance — 3.2/`07-06-a.md` §6.3; average cost —
//! 5.4/`05-06-a.md` §6.2.1; pistachio deduction — 5.6/`05-08-a.md` §8.2.3 Examples A & B).
//!
//! Each of these was already a permanent `#[sqlx::test]` in its own phase's test file
//! (`api/tests/party_balance.rs`, `api/tests/costing.rs`, `api/tests/pistachio.rs`) — this file does
//! NOT replace those (they also cover validation, permissions, and edge cases this harness doesn't
//! need to re-litigate). What it adds is the one thing the Build bullet actually asks for and those
//! files individually can't provide: a SINGLE, purpose-named, easy-to-point-at suite that IS "the
//! reconciliation harness" — the thing you run to answer "does this system's arithmetic still match
//! the documented reference values", and the thing a future populated-legacy-database migration
//! extends with real comparison data.
//!
//! **Extending this harness once 7.2's migration produces real legacy report output**: add one more
//! function in the same shape as the ones below — seed the same scenario (or reuse a migrated
//! fixture), call the real production code path, assert against the legacy system's actual output
//! instead of (or alongside) the spec's worked numbers. No restructuring: every case here already
//! follows "seed known inputs -> call the real production function/endpoint -> assert an exact
//! reference value", which is the only shape a legacy-comparison case needs too.
//!
//! **Pistachio Example C** (05-08-a.md §8.2.3 — HalfEven-vs-HalfUp rounding divergence on an exact
//! `.5` product) is deliberately NOT duplicated here: it's a pure rounding-mode identity, not a
//! report-level reconciliation, and already lives exactly where it belongs as a unit test inside
//! `api/src/money.rs`.

use api::{app, pistachio, AppState};
use axum::{
    body::Body,
    http::{header, Request, StatusCode},
};
use bigdecimal::BigDecimal;
use serde_json::{json, Value};
use sqlx::PgPool;
use tower::ServiceExt;

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

async fn seed_tenant_and_admin(pool: &PgPool) -> (i64, String) {
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
        "INSERT INTO sessions (id, user_id, tenant_id, expires_at) VALUES ($1, $2, $3, now() + interval '1 hour')",
    )
    .bind(&token)
    .bind(user_id)
    .bind(tenant_id)
    .execute(pool)
    .await
    .unwrap();
    (tenant_id, token)
}

async fn seed_account(pool: &PgPool, tenant_id: i64, gl: i32, sub: i32, a1: i32, name: &str) -> i64 {
    sqlx::query_scalar(
        "INSERT INTO accounts (tenant_id, general_ledger_code, subsidiary_code, analytic1_code, name) \
         VALUES ($1, $2, $3, $4, $5) RETURNING id",
    )
    .bind(tenant_id)
    .bind(gl)
    .bind(sub)
    .bind(a1)
    .bind(name)
    .fetch_one(pool)
    .await
    .unwrap()
}

// ---------------------------------------------------------------------------------------------
// Reference 1: party balance — 07-06-a.md §6.3, via Step 3.2's real `/parties/{id}/balance`
// endpoint (the report a user actually sees, per 10-target-architecture.md §6's own framing of
// what a reconciliation test compares).
// ---------------------------------------------------------------------------------------------

/// Reference: control accounts 103-1 (debit 50,000,000 / credit 12,000,000) and 301-1 (debit
/// 3,000,000 / credit 20,000,000) for card 52506, fiscal year 1397 -> total **-21,000,000**
/// (party is a net debtor). `07-06-a.md` §6.3.
#[sqlx::test(migrations = "./migrations")]
async fn reference_party_balance_worked_example(pool: PgPool) -> sqlx::Result<()> {
    let (tenant_id, token) = seed_tenant_and_admin(&pool).await;
    let router = app(AppState { pool: pool.clone() });
    let fiscal_year_id: i64 = sqlx::query_scalar(
        "INSERT INTO fiscal_years (tenant_id, year, start_date, end_date) \
         VALUES ($1, 1397, '2018-03-21', '2019-03-20') RETURNING id",
    )
    .bind(tenant_id)
    .fetch_one(&pool)
    .await?;
    let cash_id = seed_account(&pool, tenant_id, 1, 11, 0, "Cash").await;
    seed_account(&pool, tenant_id, 103, 0, 0, "Receivables Kol").await;
    seed_account(&pool, tenant_id, 103, 1, 0, "Trade AR").await;
    seed_account(&pool, tenant_id, 301, 0, 0, "Payables Kol").await;
    seed_account(&pool, tenant_id, 301, 1, 0, "Trade AP").await;

    let config_ar: i64 = sqlx::query_scalar(
        "INSERT INTO party_account_config \
         (tenant_id, control_kol_code, control_moein_code, name, for_person, for_legal_entity, \
          offered_by_default, counts_toward_balance) \
         VALUES ($1, 103, 1, 'Trade AR', true, false, true, true) RETURNING id",
    )
    .bind(tenant_id)
    .fetch_one(&pool)
    .await?;
    let config_ap: i64 = sqlx::query_scalar(
        "INSERT INTO party_account_config \
         (tenant_id, control_kol_code, control_moein_code, name, for_person, for_legal_entity, \
          offered_by_default, counts_toward_balance) \
         VALUES ($1, 301, 1, 'Trade AP', true, false, true, true) RETURNING id",
    )
    .bind(tenant_id)
    .fetch_one(&pool)
    .await?;

    let create = req(
        &router, "POST", "/api/v1/parties", &token,
        json!({
            "cardNumber": 52506, "partyType": "natural_person",
            "firstName": "Test", "lastName": "Party", "fatherName": "F",
            "controlAccountConfigIds": [config_ar, config_ap],
        }),
    )
    .await;
    assert_eq!(create.status(), StatusCode::CREATED);
    let party_id = json_body(create).await["id"].as_i64().unwrap();

    let leaf_103_1: i64 = sqlx::query_scalar(
        "SELECT id FROM accounts WHERE tenant_id = $1 AND general_ledger_code = 103 \
         AND subsidiary_code = 1 AND analytic1_code = 52506",
    )
    .bind(tenant_id)
    .fetch_one(&pool)
    .await?;
    let leaf_301_1: i64 = sqlx::query_scalar(
        "SELECT id FROM accounts WHERE tenant_id = $1 AND general_ledger_code = 301 \
         AND subsidiary_code = 1 AND analytic1_code = 52506",
    )
    .bind(tenant_id)
    .fetch_one(&pool)
    .await?;

    post_balanced_voucher(&router, &token, fiscal_year_id, leaf_103_1, cash_id, 50_000_000, 0).await;
    post_balanced_voucher(&router, &token, fiscal_year_id, leaf_103_1, cash_id, 0, 12_000_000).await;
    post_balanced_voucher(&router, &token, fiscal_year_id, leaf_301_1, cash_id, 3_000_000, 0).await;
    post_balanced_voucher(&router, &token, fiscal_year_id, leaf_301_1, cash_id, 0, 20_000_000).await;

    let balance = json_body(
        req(&router, "GET", &format!("/api/v1/parties/{party_id}/balance?fiscalYearId={fiscal_year_id}"), &token, Value::Null).await,
    )
    .await;
    assert_eq!(balance["total"], -21_000_000, "07-06-a.md §6.3 reference value");
    Ok(())
}

async fn post_balanced_voucher(
    router: &axum::Router,
    token: &str,
    fiscal_year_id: i64,
    account_id: i64,
    contra_account_id: i64,
    debit: i64,
    credit: i64,
) {
    let create = req(
        router, "POST", "/api/v1/vouchers", token,
        json!({ "fiscalYearId": fiscal_year_id, "voucherDate": "2018-04-01", "description": "reconciliation fixture" }),
    )
    .await;
    assert_eq!(create.status(), StatusCode::CREATED);
    let voucher_id = json_body(create).await["id"].as_i64().unwrap();

    let lines_path = format!("/api/v1/vouchers/{voucher_id}/lines");
    let add1 = req(router, "POST", &lines_path, token, json!({ "accountId": account_id, "debit": debit, "credit": credit, "description": "line" })).await;
    assert_eq!(add1.status(), StatusCode::CREATED);
    let add2 = req(router, "POST", &lines_path, token, json!({ "accountId": contra_account_id, "debit": credit, "credit": debit, "description": "line" })).await;
    assert_eq!(add2.status(), StatusCode::CREATED);

    let transition_path = format!("/api/v1/vouchers/{voucher_id}/transition");
    let confirm = req(router, "POST", &transition_path, token, json!({ "to": "confirmed" })).await;
    assert_eq!(confirm.status(), StatusCode::NO_CONTENT);
    let post = req(router, "POST", &transition_path, token, json!({ "to": "posted" })).await;
    assert_eq!(post.status(), StatusCode::NO_CONTENT);
}

// ---------------------------------------------------------------------------------------------
// Reference 2: weighted-average purchase cost — 05-06-a.md §6.2.1, via Step 5.4's real
// `/items/{id}/average-cost` endpoint.
// ---------------------------------------------------------------------------------------------

/// Reference: purchases of 100 @ 50,000, 60 @ 65,000, 40 @ 72,500 rial/kg -> average cost
/// exactly **59,000** rial/kg (`trunc(11,800,000 / 200)`, and 11,800,000/200 is exact so
/// rounding mode doesn't even enter into it). `05-06-a.md` §6.2.1.
#[sqlx::test(migrations = "./migrations")]
async fn reference_average_cost_worked_example(pool: PgPool) -> sqlx::Result<()> {
    let (tenant_id, token) = seed_tenant_and_admin(&pool).await;
    let router = app(AppState { pool: pool.clone() });
    let fiscal_year_id: i64 = sqlx::query_scalar(
        "INSERT INTO fiscal_years (tenant_id, year, start_date, end_date, is_active) \
         VALUES ($1, 1406, '2027-03-21', '2028-03-19', true) RETURNING id",
    )
    .bind(tenant_id)
    .fetch_one(&pool)
    .await?;
    let accounts: Vec<i64> = {
        let mut v = Vec::new();
        for (gl, sub, name) in [
            (900, 1, "Purchases"), (900, 2, "Purchase returns"), (901, 1, "Sales"),
            (901, 2, "Sales returns"), (902, 1, "Discounts"), (902, 2, "VAT"),
        ] {
            v.push(seed_account(&pool, tenant_id, gl, sub, 0, name).await);
        }
        v
    };
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
    let counterparty_id = seed_account(&pool, tenant_id, 103, 1, 0, "Trade AR").await;
    let uom_id = json_body(req(&router, "POST", "/api/v1/units-of-measure", &token, json!({ "name": "kg" })).await).await["id"].as_i64().unwrap();
    let item_id = json_body(
        req(&router, "POST", "/api/v1/items", &token, json!({ "code": 1, "name": "Pistachio", "unitOfMeasureId": uom_id, "salePrice": 999999 })).await,
    )
    .await["id"]
        .as_i64()
        .unwrap();

    for (date, quantity, unit_price) in [("2027-04-01", "100", 50000), ("2027-04-05", "60", 65000), ("2027-04-10", "40", 72500)] {
        let doc = req(
            &router, "POST", "/api/v1/inventory-documents", &token,
            json!({ "fiscalYearId": fiscal_year_id, "documentType": "receipt", "documentDate": date, "warehouseId": warehouse_id, "counterpartyAccountId": counterparty_id }),
        )
        .await;
        assert_eq!(doc.status(), StatusCode::CREATED);
        let doc_id = json_body(doc).await["id"].as_i64().unwrap();
        let line = req(
            &router, "POST", &format!("/api/v1/inventory-documents/{doc_id}/lines"), &token,
            json!({ "itemId": item_id, "quantity": quantity, "unitPrice": unit_price }),
        )
        .await;
        assert_eq!(line.status(), StatusCode::CREATED);
    }

    let resp = req(
        &router, "GET",
        &format!("/api/v1/items/{item_id}/average-cost?fiscalYearId={fiscal_year_id}&asOfDate=2027-12-31"),
        &token, Value::Null,
    )
    .await;
    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(json_body(resp).await["averageCost"], 59000, "05-06-a.md §6.2.1 reference value");
    Ok(())
}

// ---------------------------------------------------------------------------------------------
// Reference 3 & 4: pistachio deduction — 05-08-a.md §8.2.3 Examples A & B, calling the real
// formula function directly (`pistachio::compute_deduction` — pure, no DB, no HTTP: the "rule
// that must not drift" itself, per 10-target-architecture.md §6's own framing).
// ---------------------------------------------------------------------------------------------

fn bd(s: &str) -> BigDecimal {
    s.parse().unwrap()
}

/// Reference (Example A — ordinary lot): 40 bales @ 0.2 kg tare, 2000.0 kg gross, 3.5% moisture,
/// 2.0% blanks, 5.0 kg other, 1,250,000 rial/kg -> net weight **1877.0 kg**, line amount
/// **2,346,250,000 rial**. `05-08-a.md` §8.2.3.
#[test]
fn reference_pistachio_example_a_ordinary_lot() {
    let input = pistachio::DeductionInput {
        bale_count: 40,
        tare_allowance_kg: bd("0.2"),
        gross_weight_kg: bd("2000.0"),
        moisture_pct: bd("3.5"),
        blank_pct: bd("2.0"),
        other_deductions_kg: bd("5.0"),
        unit_price: 1_250_000,
    };
    let result = pistachio::compute_deduction(&input);
    assert_eq!(result.net_weight_kg, bd("1877.000"), "05-08-a.md §8.2.3 Example A net weight");
    assert_eq!(result.line_amount, 2_346_250_000, "05-08-a.md §8.2.3 Example A line amount");
}

/// Reference (Example B — the deduction floor): 40 bales @ 1.0 kg tare, 500.0 kg gross, 60%
/// moisture, 45% blanks, 0 other -> total deductions 565.0 kg EXCEEDS the 500.0 kg gross, net
/// weight floors to **0**, line amount **0** — and the formula does not reject or clamp
/// `total_deduction_kg` itself (shown as-is, exceeding gross). `05-08-a.md` §8.2.3.
#[test]
fn reference_pistachio_example_b_deduction_floor() {
    let input = pistachio::DeductionInput {
        bale_count: 40,
        tare_allowance_kg: bd("1.0"),
        gross_weight_kg: bd("500.0"),
        moisture_pct: bd("60"),
        blank_pct: bd("45"),
        other_deductions_kg: bd("0"),
        unit_price: 1_250_000,
    };
    let result = pistachio::compute_deduction(&input);
    assert_eq!(result.total_deduction_kg, bd("565.000"), "05-08-a.md §8.2.3 Example B: shown as-is, exceeding gross");
    assert_eq!(result.net_weight_kg, bd("0.000"), "05-08-a.md §8.2.3 Example B: net weight floors at 0");
    assert_eq!(result.line_amount, 0, "05-08-a.md §8.2.3 Example B: zero-quantity line, zero amount");
}
