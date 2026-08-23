//! Automated version of step 4.1's manual test (docs/phase-4-treasury.md
//! §4.1): the full received-cheque lifecycle, with the B11 fix (bounced is a
//! genuinely distinct, queryable state) directly exercised.

use api::{app, AppState};
use axum::{
    body::Body,
    http::{header, Request, StatusCode},
};
use serde_json::{json, Value};
use sqlx::PgPool;
use tower::ServiceExt;

struct Fixture {
    tenant_id: i64,
    fiscal_year_id: i64,
    payer_account_id: i64,
    notes_receivable_account_id: i64,
    collection_account_id: i64,
    bank_account_id: i64,
    beneficiary_account_id: i64,
    token: String,
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

    let leaf = |gl: i32, sub: i32, name: &'static str| {
        let pool = pool.clone();
        async move {
            let id: i64 = sqlx::query_scalar(
                "INSERT INTO accounts (tenant_id, general_ledger_code, subsidiary_code, name) \
                 VALUES ($1, $2, $3, $4) RETURNING id",
            )
            .bind(tenant_id)
            .bind(gl)
            .bind(sub)
            .bind(name)
            .fetch_one(&pool)
            .await
            .unwrap();
            id
        }
    };
    let payer_account_id = leaf(103, 1, "Payer").await;
    let notes_receivable_account_id = leaf(108, 1, "Notes on hand").await;
    let collection_account_id = leaf(108, 2, "Notes in collection").await;
    let bank_account_id = leaf(101, 1, "Bank").await;
    let beneficiary_account_id = leaf(103, 2, "Third party beneficiary").await;

    Fixture {
        tenant_id,
        fiscal_year_id,
        payer_account_id,
        notes_receivable_account_id,
        collection_account_id,
        bank_account_id,
        beneficiary_account_id,
        token,
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

async fn post(
    router: &axum::Router,
    token: &str,
    path: &str,
    body: Value,
) -> axum::response::Response {
    router
        .clone()
        .oneshot(
            Request::post(path)
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::COOKIE, cookie(token))
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap()
}

async fn get(router: &axum::Router, token: &str, path: &str) -> axum::response::Response {
    router
        .clone()
        .oneshot(
            Request::get(path)
                .header(header::COOKIE, cookie(token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap()
}

#[sqlx::test(migrations = "./migrations")]
async fn full_lifecycle_reaches_every_state(pool: PgPool) -> sqlx::Result<()> {
    let fx = seed(&pool).await;
    let router = app(AppState { pool: pool.clone() });

    // T1: receive -> in_hand, one event row logged from the start (unlike
    // legacy, which has no DCheck2 row for the receipt at all).
    let receive = post(
        &router,
        &fx.token,
        "/api/v1/received-cheques",
        json!({
            "fiscalYearId": fx.fiscal_year_id,
            "chequeNumber": "1001",
            "receivedOn": "2018-04-01",
            "dueDate": "2018-05-01",
            "amount": 1_000_000,
            "description": "cheque for goods",
            "payerAccountId": fx.payer_account_id,
            "notesReceivableAccountId": fx.notes_receivable_account_id,
        }),
    )
    .await;
    assert_eq!(receive.status(), StatusCode::CREATED);
    let cheque_id = json_body(receive).await["id"].as_i64().unwrap();

    let detail = json_body(
        get(
            &router,
            &fx.token,
            &format!("/api/v1/received-cheques/{cheque_id}"),
        )
        .await,
    )
    .await;
    assert_eq!(detail["status"], "in_hand");
    assert_eq!(
        detail["events"].as_array().unwrap().len(),
        1,
        "receipt itself must log an event"
    );
    assert_eq!(detail["events"][0]["resultingStatus"], "in_hand");

    // T4: deposit -> at_bank.
    let deposit = post(
        &router,
        &fx.token,
        &format!("/api/v1/received-cheques/{cheque_id}/deposit"),
        json!({
            "fiscalYearId": fx.fiscal_year_id,
            "eventDate": "2018-04-05",
            "collectionAccountId": fx.collection_account_id,
        }),
    )
    .await;
    assert_eq!(deposit.status(), StatusCode::OK);
    assert_eq!(json_body(deposit).await["status"], "at_bank");

    // T5: bounce -> bounced, a genuinely distinct state (B11 fix), NOT back
    // to in_hand the way the legacy collapses it.
    let bounce = post(
        &router,
        &fx.token,
        &format!("/api/v1/received-cheques/{cheque_id}/bounce"),
        json!({ "fiscalYearId": fx.fiscal_year_id, "eventDate": "2018-04-10" }),
    )
    .await;
    assert_eq!(bounce.status(), StatusCode::OK);
    let after_bounce = json_body(bounce).await;
    assert_eq!(after_bounce["status"], "bounced");
    assert_ne!(
        after_bounce["status"], "in_hand",
        "B11: bounced must be distinct from in_hand"
    );

    // The event log agrees with the master row (B10 fix).
    let detail2 = json_body(
        get(
            &router,
            &fx.token,
            &format!("/api/v1/received-cheques/{cheque_id}"),
        )
        .await,
    )
    .await;
    let events = detail2["events"].as_array().unwrap();
    assert_eq!(events.last().unwrap()["resultingStatus"], "bounced");
    assert_eq!(detail2["status"], events.last().unwrap()["resultingStatus"]);

    // T4 again: re-deposit the bounced cheque -> at_bank again.
    let redeposit = post(
        &router,
        &fx.token,
        &format!("/api/v1/received-cheques/{cheque_id}/deposit"),
        json!({
            "fiscalYearId": fx.fiscal_year_id,
            "eventDate": "2018-04-15",
            "collectionAccountId": fx.collection_account_id,
        }),
    )
    .await;
    assert_eq!(redeposit.status(), StatusCode::OK);
    assert_eq!(json_body(redeposit).await["status"], "at_bank");

    // T6: collect -> cleared.
    let collect = post(
        &router,
        &fx.token,
        &format!("/api/v1/received-cheques/{cheque_id}/collect"),
        json!({
            "fiscalYearId": fx.fiscal_year_id,
            "eventDate": "2018-04-20",
            "bankAccountId": fx.bank_account_id,
        }),
    )
    .await;
    assert_eq!(collect.status(), StatusCode::OK);
    assert_eq!(json_body(collect).await["status"], "cleared");

    // Terminal: any further transition is rejected.
    let illegal_deposit = post(
        &router,
        &fx.token,
        &format!("/api/v1/received-cheques/{cheque_id}/deposit"),
        json!({
            "fiscalYearId": fx.fiscal_year_id,
            "eventDate": "2018-04-21",
            "collectionAccountId": fx.collection_account_id,
        }),
    )
    .await;
    assert_eq!(illegal_deposit.status(), StatusCode::BAD_REQUEST);
    let illegal_bounce = post(
        &router,
        &fx.token,
        &format!("/api/v1/received-cheques/{cheque_id}/bounce"),
        json!({ "fiscalYearId": fx.fiscal_year_id, "eventDate": "2018-04-21" }),
    )
    .await;
    assert_eq!(illegal_bounce.status(), StatusCode::BAD_REQUEST);

    Ok(())
}

#[sqlx::test(migrations = "./migrations")]
async fn return_to_issuer_directly_from_in_hand_is_terminal(pool: PgPool) -> sqlx::Result<()> {
    let fx = seed(&pool).await;
    let router = app(AppState { pool: pool.clone() });

    let receive = post(
        &router,
        &fx.token,
        "/api/v1/received-cheques",
        json!({
            "fiscalYearId": fx.fiscal_year_id,
            "receivedOn": "2018-04-01",
            "dueDate": "2018-05-01",
            "amount": 500_000,
            "description": "second cheque",
            "payerAccountId": fx.payer_account_id,
            "notesReceivableAccountId": fx.notes_receivable_account_id,
        }),
    )
    .await;
    let cheque_id = json_body(receive).await["id"].as_i64().unwrap();

    // T7: return directly, skipping deposit entirely.
    let ret = post(
        &router,
        &fx.token,
        &format!("/api/v1/received-cheques/{cheque_id}/return-to-issuer"),
        json!({ "fiscalYearId": fx.fiscal_year_id, "eventDate": "2018-04-03" }),
    )
    .await;
    assert_eq!(ret.status(), StatusCode::OK);
    assert_eq!(json_body(ret).await["status"], "returned_to_issuer");

    // Terminal.
    let illegal = post(
        &router,
        &fx.token,
        &format!("/api/v1/received-cheques/{cheque_id}/deposit"),
        json!({
            "fiscalYearId": fx.fiscal_year_id,
            "eventDate": "2018-04-04",
            "collectionAccountId": fx.collection_account_id,
        }),
    )
    .await;
    assert_eq!(illegal.status(), StatusCode::BAD_REQUEST);

    Ok(())
}

#[sqlx::test(migrations = "./migrations")]
async fn deposit_requires_leaf_collection_account(pool: PgPool) -> sqlx::Result<()> {
    let fx = seed(&pool).await;
    let router = app(AppState { pool: pool.clone() });

    // A non-leaf: give the collection account's Moein a fake child so
    // child_count > 0.
    sqlx::query("UPDATE accounts SET child_count = 1 WHERE id = $1")
        .bind(fx.collection_account_id)
        .execute(&pool)
        .await?;

    let receive = post(
        &router,
        &fx.token,
        "/api/v1/received-cheques",
        json!({
            "fiscalYearId": fx.fiscal_year_id,
            "receivedOn": "2018-04-01",
            "dueDate": "2018-05-01",
            "amount": 100_000,
            "description": "d",
            "payerAccountId": fx.payer_account_id,
            "notesReceivableAccountId": fx.notes_receivable_account_id,
        }),
    )
    .await;
    let cheque_id = json_body(receive).await["id"].as_i64().unwrap();

    let deposit = post(
        &router,
        &fx.token,
        &format!("/api/v1/received-cheques/{cheque_id}/deposit"),
        json!({
            "fiscalYearId": fx.fiscal_year_id,
            "eventDate": "2018-04-05",
            "collectionAccountId": fx.collection_account_id,
        }),
    )
    .await;
    assert_eq!(deposit.status(), StatusCode::BAD_REQUEST);
    assert_eq!(json_body(deposit).await["error"], "account_not_leaf");

    Ok(())
}

#[sqlx::test(migrations = "./migrations")]
async fn receive_validates_amount_description_and_date_range(pool: PgPool) -> sqlx::Result<()> {
    let fx = seed(&pool).await;
    let router = app(AppState { pool: pool.clone() });

    let bad_amount = post(
        &router,
        &fx.token,
        "/api/v1/received-cheques",
        json!({
            "fiscalYearId": fx.fiscal_year_id,
            "receivedOn": "2018-04-01",
            "dueDate": "2018-05-01",
            "amount": 0,
            "description": "x",
            "payerAccountId": fx.payer_account_id,
            "notesReceivableAccountId": fx.notes_receivable_account_id,
        }),
    )
    .await;
    assert_eq!(bad_amount.status(), StatusCode::BAD_REQUEST);
    assert_eq!(
        json_body(bad_amount).await["error"],
        "amount_must_be_positive"
    );

    let blank_desc = post(
        &router,
        &fx.token,
        "/api/v1/received-cheques",
        json!({
            "fiscalYearId": fx.fiscal_year_id,
            "receivedOn": "2018-04-01",
            "dueDate": "2018-05-01",
            "amount": 1000,
            "description": "   ",
            "payerAccountId": fx.payer_account_id,
            "notesReceivableAccountId": fx.notes_receivable_account_id,
        }),
    )
    .await;
    assert_eq!(blank_desc.status(), StatusCode::BAD_REQUEST);
    assert_eq!(json_body(blank_desc).await["error"], "description_required");

    let outside_range = post(
        &router,
        &fx.token,
        "/api/v1/received-cheques",
        json!({
            "fiscalYearId": fx.fiscal_year_id,
            "receivedOn": "2020-01-01",
            "dueDate": "2020-02-01",
            "amount": 1000,
            "description": "x",
            "payerAccountId": fx.payer_account_id,
            "notesReceivableAccountId": fx.notes_receivable_account_id,
        }),
    )
    .await;
    assert_eq!(outside_range.status(), StatusCode::BAD_REQUEST);
    assert_eq!(
        json_body(outside_range).await["error"],
        "date_outside_fiscal_year"
    );

    Ok(())
}

#[sqlx::test(migrations = "./migrations")]
async fn amend_only_allowed_while_in_hand(pool: PgPool) -> sqlx::Result<()> {
    let fx = seed(&pool).await;
    let router = app(AppState { pool: pool.clone() });

    let receive = post(
        &router,
        &fx.token,
        "/api/v1/received-cheques",
        json!({
            "fiscalYearId": fx.fiscal_year_id,
            "receivedOn": "2018-04-01",
            "dueDate": "2018-05-01",
            "amount": 1_000_000,
            "description": "original",
            "payerAccountId": fx.payer_account_id,
            "notesReceivableAccountId": fx.notes_receivable_account_id,
        }),
    )
    .await;
    let cheque_id = json_body(receive).await["id"].as_i64().unwrap();

    // Edit while in_hand succeeds.
    let amend = router
        .clone()
        .oneshot(
            Request::put(format!("/api/v1/received-cheques/{cheque_id}"))
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::COOKIE, cookie(&fx.token))
                .body(Body::from(
                    json!({
                        "fiscalYearId": fx.fiscal_year_id,
                        "receivedOn": "2018-04-01",
                        "dueDate": "2018-05-10",
                        "amount": 1_200_000,
                        "description": "amended",
                        "payerAccountId": fx.payer_account_id,
                        "notesReceivableAccountId": fx.notes_receivable_account_id,
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(amend.status(), StatusCode::NO_CONTENT);
    let after = json_body(
        get(
            &router,
            &fx.token,
            &format!("/api/v1/received-cheques/{cheque_id}"),
        )
        .await,
    )
    .await;
    assert_eq!(after["amount"], 1_200_000);

    // Deposit, then editing should be rejected (no longer in_hand).
    post(
        &router,
        &fx.token,
        &format!("/api/v1/received-cheques/{cheque_id}/deposit"),
        json!({
            "fiscalYearId": fx.fiscal_year_id,
            "eventDate": "2018-04-05",
            "collectionAccountId": fx.collection_account_id,
        }),
    )
    .await;

    let amend_after_deposit = router
        .oneshot(
            Request::put(format!("/api/v1/received-cheques/{cheque_id}"))
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::COOKIE, cookie(&fx.token))
                .body(Body::from(
                    json!({
                        "fiscalYearId": fx.fiscal_year_id,
                        "receivedOn": "2018-04-01",
                        "dueDate": "2018-05-10",
                        "amount": 1_300_000,
                        "description": "should fail",
                        "payerAccountId": fx.payer_account_id,
                        "notesReceivableAccountId": fx.notes_receivable_account_id,
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(amend_after_deposit.status(), StatusCode::BAD_REQUEST);
    assert_eq!(
        json_body(amend_after_deposit).await["error"],
        "cheque_not_in_hand"
    );

    Ok(())
}

async fn delete(router: &axum::Router, token: &str, path: &str) -> axum::response::Response {
    router
        .clone()
        .oneshot(
            Request::delete(path)
                .header(header::COOKIE, cookie(token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap()
}

/// Step 4.2's manual test #1: receive, deposit, collect -> three distinct
/// vouchers exist (not one shared daily voucher, B13 fix), each balanced.
#[sqlx::test(migrations = "./migrations")]
async fn each_transition_posts_its_own_balanced_voucher(pool: PgPool) -> sqlx::Result<()> {
    let fx = seed(&pool).await;
    let router = app(AppState { pool: pool.clone() });

    let receive = post(
        &router,
        &fx.token,
        "/api/v1/received-cheques",
        json!({
            "fiscalYearId": fx.fiscal_year_id,
            "chequeNumber": "5001",
            "receivedOn": "2018-04-01",
            "dueDate": "2018-05-01",
            "amount": 750_000,
            "description": "B13 regression",
            "payerAccountId": fx.payer_account_id,
            "notesReceivableAccountId": fx.notes_receivable_account_id,
        }),
    )
    .await;
    let cheque_id = json_body(receive).await["id"].as_i64().unwrap();
    let receipt_voucher_id = json_body(
        get(
            &router,
            &fx.token,
            &format!("/api/v1/received-cheques/{cheque_id}"),
        )
        .await,
    )
    .await["voucherId"]
        .as_i64()
        .unwrap();

    let deposit = post(
        &router,
        &fx.token,
        &format!("/api/v1/received-cheques/{cheque_id}/deposit"),
        json!({
            "fiscalYearId": fx.fiscal_year_id,
            "eventDate": "2018-04-05",
            "collectionAccountId": fx.collection_account_id,
        }),
    )
    .await;
    assert_eq!(deposit.status(), StatusCode::OK);

    let collect = post(
        &router,
        &fx.token,
        &format!("/api/v1/received-cheques/{cheque_id}/collect"),
        json!({
            "fiscalYearId": fx.fiscal_year_id,
            "eventDate": "2018-04-20",
            "bankAccountId": fx.bank_account_id,
        }),
    )
    .await;
    assert_eq!(collect.status(), StatusCode::OK);

    let detail = json_body(
        get(
            &router,
            &fx.token,
            &format!("/api/v1/received-cheques/{cheque_id}"),
        )
        .await,
    )
    .await;
    let events = detail["events"].as_array().unwrap();
    assert_eq!(
        events.len(),
        3,
        "receive, deposit, collect each log one event"
    );

    let deposit_voucher_id = events[1]["voucherId"]
        .as_i64()
        .expect("deposit event must have a voucher (B13)");
    let collect_voucher_id = events[2]["voucherId"]
        .as_i64()
        .expect("collect event must have a voucher — B13 fix, exactly what the legacy omits");

    // Three DISTINCT vouchers, not one shared daily voucher.
    assert_ne!(receipt_voucher_id, deposit_voucher_id);
    assert_ne!(deposit_voucher_id, collect_voucher_id);
    assert_ne!(receipt_voucher_id, collect_voucher_id);

    // Each voucher is balanced (debit == credit) and has a real header.
    for voucher_id in [receipt_voucher_id, deposit_voucher_id, collect_voucher_id] {
        let (total_debit, total_credit, line_count): (i64, i64, i32) = sqlx::query_as(
            "SELECT total_debit, total_credit, line_count FROM vouchers WHERE id = $1",
        )
        .bind(voucher_id)
        .fetch_one(&pool)
        .await?;
        assert_eq!(
            total_debit, total_credit,
            "voucher {voucher_id} must balance"
        );
        assert_eq!(total_debit, 750_000);
        assert_eq!(line_count, 2);
    }

    Ok(())
}

/// Step 4.2's manual test #2: bounce a deposited cheque, then confirm the
/// event log and the cheque's current state agree (B10 fix).
#[sqlx::test(migrations = "./migrations")]
async fn bounce_event_and_master_state_agree(pool: PgPool) -> sqlx::Result<()> {
    let fx = seed(&pool).await;
    let router = app(AppState { pool: pool.clone() });

    let receive = post(
        &router,
        &fx.token,
        "/api/v1/received-cheques",
        json!({
            "fiscalYearId": fx.fiscal_year_id,
            "receivedOn": "2018-04-01",
            "dueDate": "2018-05-01",
            "amount": 300_000,
            "description": "B10 regression",
            "payerAccountId": fx.payer_account_id,
            "notesReceivableAccountId": fx.notes_receivable_account_id,
        }),
    )
    .await;
    let cheque_id = json_body(receive).await["id"].as_i64().unwrap();

    post(
        &router,
        &fx.token,
        &format!("/api/v1/received-cheques/{cheque_id}/deposit"),
        json!({
            "fiscalYearId": fx.fiscal_year_id,
            "eventDate": "2018-04-05",
            "collectionAccountId": fx.collection_account_id,
        }),
    )
    .await;
    post(
        &router,
        &fx.token,
        &format!("/api/v1/received-cheques/{cheque_id}/bounce"),
        json!({ "fiscalYearId": fx.fiscal_year_id, "eventDate": "2018-04-10" }),
    )
    .await;

    let (master_status,): (String,) =
        sqlx::query_as("SELECT status::text FROM received_cheques WHERE id = $1")
            .bind(cheque_id)
            .fetch_one(&pool)
            .await?;
    let (event_status,): (String,) = sqlx::query_as(
        "SELECT resulting_status::text FROM received_cheque_events \
         WHERE received_cheque_id = $1 ORDER BY id DESC LIMIT 1",
    )
    .bind(cheque_id)
    .fetch_one(&pool)
    .await?;
    assert_eq!(master_status, "bounced");
    assert_eq!(event_status, "bounced");
    assert_eq!(
        master_status, event_status,
        "B10: master and event must never disagree"
    );

    Ok(())
}

/// Step 4.2's manual test #3-4 (B12 fix): a freshly-received cheque can
/// really be deleted (cheque, voucher and event all gone); a deposited one
/// cannot.
#[sqlx::test(migrations = "./migrations")]
async fn delete_only_allowed_before_any_transition(pool: PgPool) -> sqlx::Result<()> {
    let fx = seed(&pool).await;
    let router = app(AppState { pool: pool.clone() });

    let receive = post(
        &router,
        &fx.token,
        "/api/v1/received-cheques",
        json!({
            "fiscalYearId": fx.fiscal_year_id,
            "receivedOn": "2018-04-01",
            "dueDate": "2018-05-01",
            "amount": 400_000,
            "description": "B12 regression -- deletable",
            "payerAccountId": fx.payer_account_id,
            "notesReceivableAccountId": fx.notes_receivable_account_id,
        }),
    )
    .await;
    let cheque_id = json_body(receive).await["id"].as_i64().unwrap();
    let voucher_id = json_body(
        get(
            &router,
            &fx.token,
            &format!("/api/v1/received-cheques/{cheque_id}"),
        )
        .await,
    )
    .await["voucherId"]
        .as_i64()
        .unwrap();

    let del = delete(
        &router,
        &fx.token,
        &format!("/api/v1/received-cheques/{cheque_id}"),
    )
    .await;
    assert_eq!(del.status(), StatusCode::NO_CONTENT);

    let after = get(
        &router,
        &fx.token,
        &format!("/api/v1/received-cheques/{cheque_id}"),
    )
    .await;
    assert_eq!(
        after.status(),
        StatusCode::NOT_FOUND,
        "cheque itself must be gone"
    );
    let voucher_gone: Option<i64> = sqlx::query_scalar("SELECT id FROM vouchers WHERE id = $1")
        .bind(voucher_id)
        .fetch_optional(&pool)
        .await?;
    assert!(voucher_gone.is_none(), "voucher must be gone too (B12)");
    let events_gone: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM received_cheque_events WHERE received_cheque_id = $1",
    )
    .bind(cheque_id)
    .fetch_one(&pool)
    .await?;
    assert_eq!(events_gone, 0, "event row must be gone too (B12)");

    // A second cheque, deposited, is NOT deletable.
    let receive2 = post(
        &router,
        &fx.token,
        "/api/v1/received-cheques",
        json!({
            "fiscalYearId": fx.fiscal_year_id,
            "receivedOn": "2018-04-01",
            "dueDate": "2018-05-01",
            "amount": 400_000,
            "description": "B12 regression -- not deletable",
            "payerAccountId": fx.payer_account_id,
            "notesReceivableAccountId": fx.notes_receivable_account_id,
        }),
    )
    .await;
    let cheque_id2 = json_body(receive2).await["id"].as_i64().unwrap();
    post(
        &router,
        &fx.token,
        &format!("/api/v1/received-cheques/{cheque_id2}/deposit"),
        json!({
            "fiscalYearId": fx.fiscal_year_id,
            "eventDate": "2018-04-05",
            "collectionAccountId": fx.collection_account_id,
        }),
    )
    .await;

    let del2 = delete(
        &router,
        &fx.token,
        &format!("/api/v1/received-cheques/{cheque_id2}"),
    )
    .await;
    assert_eq!(del2.status(), StatusCode::BAD_REQUEST);
    // Rejected at the status check first (no longer in_hand) -- the
    // event-count check below it exists for an in_hand cheque with extra
    // history, which a deposited cheque never is.
    let body2 = json_body(del2).await;
    assert_eq!(body2["error"], "cheque_not_deletable");

    Ok(())
}

/// Step 4.3's manual test: receive, endorse to a third-party account ->
/// terminal EndorsedToThirdParty, with a real posted voucher debiting the
/// beneficiary and crediting notes-receivable-on-hand -- a genuine new
/// feature (B14), never present in the legacy at all.
#[sqlx::test(migrations = "./migrations")]
async fn endorsement_is_a_real_terminal_transition_with_a_real_posting(
    pool: PgPool,
) -> sqlx::Result<()> {
    let fx = seed(&pool).await;
    let router = app(AppState { pool: pool.clone() });

    let receive = post(
        &router,
        &fx.token,
        "/api/v1/received-cheques",
        json!({
            "fiscalYearId": fx.fiscal_year_id,
            "chequeNumber": "8001",
            "receivedOn": "2018-04-01",
            "dueDate": "2018-05-01",
            "amount": 600_000,
            "description": "B14 regression",
            "payerAccountId": fx.payer_account_id,
            "notesReceivableAccountId": fx.notes_receivable_account_id,
        }),
    )
    .await;
    let cheque_id = json_body(receive).await["id"].as_i64().unwrap();

    let endorse = post(
        &router,
        &fx.token,
        &format!("/api/v1/received-cheques/{cheque_id}/endorse"),
        json!({
            "fiscalYearId": fx.fiscal_year_id,
            "eventDate": "2018-04-03",
            "beneficiaryAccountId": fx.beneficiary_account_id,
        }),
    )
    .await;
    assert_eq!(endorse.status(), StatusCode::OK);
    let after = json_body(endorse).await;
    assert_eq!(after["status"], "endorsed_to_third_party");

    let detail = json_body(
        get(
            &router,
            &fx.token,
            &format!("/api/v1/received-cheques/{cheque_id}"),
        )
        .await,
    )
    .await;
    let events = detail["events"].as_array().unwrap();
    let endorse_event = events.last().unwrap();
    assert_eq!(endorse_event["resultingStatus"], "endorsed_to_third_party");
    assert_eq!(endorse_event["debitAccountId"], fx.beneficiary_account_id);
    assert_eq!(
        endorse_event["creditAccountId"],
        fx.notes_receivable_account_id
    );

    let voucher_id = endorse_event["voucherId"]
        .as_i64()
        .expect("endorsement must post a real voucher");
    let (total_debit, total_credit): (i64, i64) =
        sqlx::query_as("SELECT total_debit, total_credit FROM vouchers WHERE id = $1")
            .bind(voucher_id)
            .fetch_one(&pool)
            .await?;
    assert_eq!(total_debit, total_credit);
    assert_eq!(total_debit, 600_000);
    let debit_line_account: i64 = sqlx::query_scalar(
        "SELECT account_id FROM voucher_lines WHERE voucher_id = $1 AND debit_amount > 0",
    )
    .bind(voucher_id)
    .fetch_one(&pool)
    .await?;
    assert_eq!(
        debit_line_account, fx.beneficiary_account_id,
        "voucher must debit the beneficiary"
    );
    let credit_line_account: i64 = sqlx::query_scalar(
        "SELECT account_id FROM voucher_lines WHERE voucher_id = $1 AND credit_amount > 0",
    )
    .bind(voucher_id)
    .fetch_one(&pool)
    .await?;
    assert_eq!(
        credit_line_account, fx.notes_receivable_account_id,
        "voucher must credit notes-receivable-on-hand"
    );

    // Terminal: any further transition is rejected.
    let illegal = post(
        &router,
        &fx.token,
        &format!("/api/v1/received-cheques/{cheque_id}/deposit"),
        json!({
            "fiscalYearId": fx.fiscal_year_id,
            "eventDate": "2018-04-04",
            "collectionAccountId": fx.collection_account_id,
        }),
    )
    .await;
    assert_eq!(illegal.status(), StatusCode::BAD_REQUEST);

    Ok(())
}

/// Step 4.5's manual test #1-2 (B15 fix): status filter and due-date aging
/// both actually filter -- unlike the legacy where the equivalent controls
/// exist in the query but are unreachable from the UI (§5.5).
#[sqlx::test(migrations = "./migrations")]
async fn status_and_aging_filters_actually_filter(pool: PgPool) -> sqlx::Result<()> {
    let fx = seed(&pool).await;
    let router = app(AppState { pool: pool.clone() });

    // Cheque A: stays in_hand, due date early -- should appear in the aging cutoff.
    let a = post(
        &router,
        &fx.token,
        "/api/v1/received-cheques",
        json!({
            "fiscalYearId": fx.fiscal_year_id, "chequeNumber": "A", "receivedOn": "2018-04-01",
            "dueDate": "2018-04-10", "amount": 100_000, "description": "a",
            "payerAccountId": fx.payer_account_id, "notesReceivableAccountId": fx.notes_receivable_account_id,
        }),
    )
    .await;
    let a_id = json_body(a).await["id"].as_i64().unwrap();

    // Cheque B: due date early too, but returned -- must be EXCLUDED from aging
    // (a terminal exit, matching the legacy's `S_State<4` clause).
    let b = post(
        &router,
        &fx.token,
        "/api/v1/received-cheques",
        json!({
            "fiscalYearId": fx.fiscal_year_id, "chequeNumber": "B", "receivedOn": "2018-04-01",
            "dueDate": "2018-04-05", "amount": 200_000, "description": "b",
            "payerAccountId": fx.payer_account_id, "notesReceivableAccountId": fx.notes_receivable_account_id,
        }),
    )
    .await;
    let b_id = json_body(b).await["id"].as_i64().unwrap();
    post(
        &router,
        &fx.token,
        &format!("/api/v1/received-cheques/{b_id}/return-to-issuer"),
        json!({ "fiscalYearId": fx.fiscal_year_id, "eventDate": "2018-04-02" }),
    )
    .await;

    // Cheque C: due date far in the future -- must be excluded by the cutoff.
    let c = post(
        &router,
        &fx.token,
        "/api/v1/received-cheques",
        json!({
            "fiscalYearId": fx.fiscal_year_id, "chequeNumber": "C", "receivedOn": "2018-04-01",
            "dueDate": "2019-01-01", "amount": 300_000, "description": "c",
            "payerAccountId": fx.payer_account_id, "notesReceivableAccountId": fx.notes_receivable_account_id,
        }),
    )
    .await;
    let _c_id = json_body(c).await["id"].as_i64().unwrap();

    // Status filter: only A and C are in_hand; B is returned_to_issuer.
    let by_status = json_body(
        get(
            &router,
            &fx.token,
            "/api/v1/received-cheques?status=in_hand",
        )
        .await,
    )
    .await;
    let status_ids: Vec<i64> = by_status
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["id"].as_i64().unwrap())
        .collect();
    assert!(status_ids.contains(&a_id));
    assert!(!status_ids.contains(&b_id));

    let by_returned = json_body(
        get(
            &router,
            &fx.token,
            "/api/v1/received-cheques?status=returned_to_issuer",
        )
        .await,
    )
    .await;
    let returned_ids: Vec<i64> = by_returned
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["id"].as_i64().unwrap())
        .collect();
    assert_eq!(returned_ids, vec![b_id]);

    // Aging cutoff 2018-04-15: A (due 2018-04-10, in_hand) qualifies; B (due
    // 2018-04-05 but returned) must be excluded even though its due date is
    // before the cutoff; C (due 2019-01-01) is excluded by date.
    let aging = json_body(
        get(
            &router,
            &fx.token,
            "/api/v1/received-cheques?dueBefore=2018-04-15",
        )
        .await,
    )
    .await;
    let aging_ids: Vec<i64> = aging
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["id"].as_i64().unwrap())
        .collect();
    assert_eq!(
        aging_ids,
        vec![a_id],
        "aging must include A only -- B excluded by terminal status, C by date"
    );

    Ok(())
}
