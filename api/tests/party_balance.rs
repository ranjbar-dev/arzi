//! Automated version of step 3.2's manual test (docs/phase-3-parties.md
//! §3.2): reproduces 07-06-a.md §6.3's worked example exactly, including the
//! Tafsil-2/fixed-Tafsil1 extension the legacy's write path can never reach.

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
    cash_account_id: i64, // contra leg for every posting below
    token: String,
}

async fn seed(pool: &PgPool) -> Fixture {
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
    let fiscal_year_id: i64 = sqlx::query_scalar(
        "INSERT INTO fiscal_years (tenant_id, year, start_date, end_date) \
         VALUES ($1, 1397, '2018-03-21', '2019-03-20') RETURNING id",
    )
    .bind(tenant_id)
    .fetch_one(pool)
    .await
    .unwrap();
    let cash_account_id: i64 = sqlx::query_scalar(
        "INSERT INTO accounts (tenant_id, general_ledger_code, subsidiary_code, name) \
         VALUES ($1, 1, 11, 'Cash') RETURNING id",
    )
    .bind(tenant_id)
    .fetch_one(pool)
    .await
    .unwrap();
    Fixture { tenant_id, fiscal_year_id, cash_account_id, token }
}

fn cookie(token: &str) -> String {
    format!("arzi_session={token}")
}

async fn json_body(response: axum::response::Response) -> Value {
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

async fn seed_account(pool: &PgPool, tenant_id: i64, gl: i32, sub: i32, a1: i32, a2: i32, name: &str) -> i64 {
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

async fn seed_config(pool: &PgPool, tenant_id: i64, kol: i32, moein: i32, fixed_ta1: i32, name: &str) -> i64 {
    sqlx::query_scalar(
        "INSERT INTO party_account_config \
         (tenant_id, control_kol_code, control_moein_code, fixed_tafsil1_code, name, for_person, \
          for_legal_entity, offered_by_default, counts_toward_balance) \
         VALUES ($1, $2, $3, $4, $5, true, false, true, true) RETURNING id",
    )
    .bind(tenant_id)
    .bind(kol)
    .bind(moein)
    .bind(fixed_ta1)
    .bind(name)
    .fetch_one(pool)
    .await
    .unwrap()
}

/// Posts a fully balanced, two-line voucher (the account leg + a Cash
/// contra leg) straight through draft -> confirmed -> posted.
async fn post_voucher(
    router: &axum::Router,
    token: &str,
    fiscal_year_id: i64,
    account_id: i64,
    cash_account_id: i64,
    account_debit: i64,
    account_credit: i64,
) {
    let create = router
        .clone()
        .oneshot(
            Request::post("/api/v1/vouchers")
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::COOKIE, cookie(token))
                .body(Body::from(
                    json!({
                        "fiscalYearId": fiscal_year_id,
                        "voucherDate": "2018-04-01",
                        "description": "Jari worked example"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(create.status(), StatusCode::CREATED);
    let voucher_id = json_body(create).await["id"].as_i64().unwrap();

    let add = |acc: i64, debit: i64, credit: i64| {
        let router = router.clone();
        let token = token.to_string();
        async move {
            router
                .oneshot(
                    Request::post(format!("/api/v1/vouchers/{voucher_id}/lines"))
                        .header(header::CONTENT_TYPE, "application/json")
                        .header(header::COOKIE, cookie(&token))
                        .body(Body::from(
                            json!({ "accountId": acc, "debit": debit, "credit": credit, "description": "line" })
                                .to_string(),
                        ))
                        .unwrap(),
                )
                .await
                .unwrap()
        }
    };
    assert_eq!(add(account_id, account_debit, account_credit).await.status(), StatusCode::CREATED);
    assert_eq!(add(cash_account_id, account_credit, account_debit).await.status(), StatusCode::CREATED);

    let transition = |to: &'static str| {
        let router = router.clone();
        let token = token.to_string();
        async move {
            router
                .oneshot(
                    Request::post(format!("/api/v1/vouchers/{voucher_id}/transition"))
                        .header(header::CONTENT_TYPE, "application/json")
                        .header(header::COOKIE, cookie(&token))
                        .body(Body::from(json!({ "to": to }).to_string()))
                        .unwrap(),
                )
                .await
                .unwrap()
        }
    };
    assert_eq!(transition("confirmed").await.status(), StatusCode::NO_CONTENT);
    assert_eq!(transition("posted").await.status(), StatusCode::NO_CONTENT);
}

#[sqlx::test(migrations = "./migrations")]
async fn reproduces_the_worked_example_exactly(pool: PgPool) -> sqlx::Result<()> {
    let fx = seed(&pool).await;
    let router = app(AppState { pool: pool.clone() });

    // Control accounts from 07-06-a.md §6.3: 103-1, 104-1 (stays untouched),
    // 301-1, all Tafsil-1-only.
    seed_account(&pool, fx.tenant_id, 103, 0, 0, 0, "Receivables Kol").await;
    seed_account(&pool, fx.tenant_id, 103, 1, 0, 0, "Trade AR").await;
    seed_account(&pool, fx.tenant_id, 301, 0, 0, 0, "Payables Kol").await;
    seed_account(&pool, fx.tenant_id, 301, 1, 0, 0, "Trade AP").await;
    let config_103_1 = seed_config(&pool, fx.tenant_id, 103, 1, 0, "Trade AR — persons").await;
    let config_301_1 = seed_config(&pool, fx.tenant_id, 301, 1, 0, "Trade AP — persons").await;

    let create_party = router
        .clone()
        .oneshot(
            Request::post("/api/v1/parties")
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::COOKIE, cookie(&fx.token))
                .body(Body::from(
                    json!({
                        "cardNumber": 52506, "partyType": "natural_person",
                        "firstName": "Test", "lastName": "Party", "fatherName": "F",
                        "controlAccountConfigIds": [config_103_1, config_301_1],
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(create_party.status(), StatusCode::CREATED);
    let party_id = json_body(create_party).await["id"].as_i64().unwrap();

    let leaf_103_1: i64 = sqlx::query_scalar(
        "SELECT id FROM accounts WHERE tenant_id = $1 AND general_ledger_code = 103 \
         AND subsidiary_code = 1 AND analytic1_code = 52506",
    )
    .bind(fx.tenant_id)
    .fetch_one(&pool)
    .await?;
    let leaf_301_1: i64 = sqlx::query_scalar(
        "SELECT id FROM accounts WHERE tenant_id = $1 AND general_ledger_code = 301 \
         AND subsidiary_code = 1 AND analytic1_code = 52506",
    )
    .bind(fx.tenant_id)
    .fetch_one(&pool)
    .await?;

    // (103,1,52506,0): debit 50,000,000 / credit 12,000,000 -> Rem = -38,000,000.
    post_voucher(&router, &fx.token, fx.fiscal_year_id, leaf_103_1, fx.cash_account_id, 50_000_000, 0).await;
    post_voucher(&router, &fx.token, fx.fiscal_year_id, leaf_103_1, fx.cash_account_id, 0, 12_000_000).await;
    // (301,1,52506,0): debit 3,000,000 / credit 20,000,000 -> Rem = +17,000,000.
    post_voucher(&router, &fx.token, fx.fiscal_year_id, leaf_301_1, fx.cash_account_id, 3_000_000, 0).await;
    post_voucher(&router, &fx.token, fx.fiscal_year_id, leaf_301_1, fx.cash_account_id, 0, 20_000_000).await;

    let balance_resp = router
        .clone()
        .oneshot(
            Request::get(format!("/api/v1/parties/{party_id}/balance?fiscalYearId={}", fx.fiscal_year_id))
                .header(header::COOKIE, cookie(&fx.token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(balance_resp.status(), StatusCode::OK);
    let balance = json_body(balance_resp).await;
    assert_eq!(balance["total"], -21_000_000, "party 52506 should be a net debtor of 21,000,000");
    // 104-1 was never posted to -> dropped from the breakdown (step 5).
    assert_eq!(balance["breakdown"].as_array().unwrap().len(), 2);

    // Second worked case: a fixed-Tafsil1 config (103, 9, fixed=77), credit
    // 5,000,000 -> Rem = +5,000,000, total becomes -16,000,000. Exercises the
    // Tafsil-2 path the legacy's write side can never reach (07-06-a.md §6.3).
    seed_account(&pool, fx.tenant_id, 103, 9, 0, 0, "Fixed control Moein").await;
    let fixed_ta1_id = seed_account(&pool, fx.tenant_id, 103, 9, 77, 0, "Fixed Tafsil1 node").await;
    let config_fixed = seed_config(&pool, fx.tenant_id, 103, 9, 77, "Fixed-Tafsil1 case").await;

    let update_party = router
        .clone()
        .oneshot(
            Request::put(format!("/api/v1/parties/{party_id}"))
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::COOKIE, cookie(&fx.token))
                .body(Body::from(
                    json!({
                        "cardNumber": 52506, "partyType": "natural_person",
                        "firstName": "Test", "lastName": "Party", "fatherName": "F",
                        "controlAccountConfigIds": [config_103_1, config_301_1, config_fixed],
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(update_party.status(), StatusCode::NO_CONTENT);

    let leaf_fixed: i64 = sqlx::query_scalar(
        "SELECT id FROM accounts WHERE tenant_id = $1 AND general_ledger_code = 103 \
         AND subsidiary_code = 9 AND analytic1_code = 77 AND analytic2_code = 52506",
    )
    .bind(fx.tenant_id)
    .fetch_one(&pool)
    .await?;
    assert_eq!(
        sqlx::query_scalar::<_, i32>("SELECT child_count FROM accounts WHERE id = $1")
            .bind(fixed_ta1_id)
            .fetch_one(&pool)
            .await?,
        1
    );

    post_voucher(&router, &fx.token, fx.fiscal_year_id, leaf_fixed, fx.cash_account_id, 0, 5_000_000).await;

    let balance_resp2 = router
        .clone()
        .oneshot(
            Request::get(format!("/api/v1/parties/{party_id}/balance?fiscalYearId={}", fx.fiscal_year_id))
                .header(header::COOKIE, cookie(&fx.token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let balance2 = json_body(balance_resp2).await;
    assert_eq!(balance2["total"], -16_000_000);
    assert_eq!(balance2["breakdown"].as_array().unwrap().len(), 3);

    // Step 6.3 (Card Jari): reuses the exact same balance the endpoint above
    // just proved, no separate reimplementation -- the manual test's own
    // "confirm the total matches exactly" for both worked-example figures.
    let card_jari_resp = router
        .clone()
        .oneshot(
            Request::get(format!("/api/v1/parties/{party_id}/card-jari?fiscalYearId={}", fx.fiscal_year_id))
                .header(header::COOKIE, cookie(&fx.token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(card_jari_resp.status(), StatusCode::OK);
    let card_jari = json_body(card_jari_resp).await;
    assert_eq!(card_jari["total"], -16_000_000, "Card Jari and the 3.2 balance API must agree exactly");
    let breakdown = card_jari["breakdown"].as_array().unwrap();
    assert_eq!(breakdown.len(), 3);

    // Drill-through: each row's ledgerHref points at 6.2's ledger endpoint,
    // scoped to that row's own account and the party's fiscal year -- and
    // actually resolves to a working, correctly-scoped ledger.
    let row = breakdown.iter().find(|r| r["accountId"] == leaf_103_1).expect("leaf_103_1 row present");
    let href = row["ledgerHref"].as_str().unwrap();
    assert!(href.contains(&format!("accountIds={leaf_103_1}")));
    let ledger_resp = router
        .oneshot(Request::get(href).header(header::COOKIE, cookie(&fx.token)).body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(ledger_resp.status(), StatusCode::OK);
    let ledger = json_body(ledger_resp).await;
    // leaf_103_1 alone: debit 50,000,000 / credit 12,000,000 -> net -38,000,000 (debit).
    assert_eq!(ledger["closingBalance"], json!({ "amount": 38_000_000, "side": "debit" }));

    Ok(())
}
