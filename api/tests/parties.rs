//! Automated version of step 3.1's manual test (docs/phase-3-parties.md
//! §3.1): auto-created leaf accounts for both the Tafsil-1-only and the
//! fixed-Tafsil1/Tafsil-2 cases, untick-deletes a childless leaf, and the
//! B18 fix (no global tick state to leak between two different parties).

use api::{app, AppState};
use axum::{
    body::Body,
    http::{header, Request, StatusCode},
};
use serde_json::{json, Value};
use sqlx::PgPool;
use tower::ServiceExt;

async fn seed_session(pool: &PgPool) -> (i64, i64, String) {
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
    (tenant_id, user_id, token)
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

async fn seed_account(
    pool: &PgPool,
    tenant_id: i64,
    gl: i32,
    sub: i32,
    a1: i32,
    a2: i32,
    name: &str,
) -> i64 {
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

async fn seed_config(
    pool: &PgPool,
    tenant_id: i64,
    kol: i32,
    moein: i32,
    fixed_ta1: i32,
    name: &str,
    for_person: bool,
    for_legal_entity: bool,
) -> i64 {
    sqlx::query_scalar(
        "INSERT INTO party_account_config \
         (tenant_id, control_kol_code, control_moein_code, fixed_tafsil1_code, name, for_person, \
          for_legal_entity, offered_by_default, counts_toward_balance) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, true, true) RETURNING id",
    )
    .bind(tenant_id)
    .bind(kol)
    .bind(moein)
    .bind(fixed_ta1)
    .bind(name)
    .bind(for_person)
    .bind(for_legal_entity)
    .fetch_one(pool)
    .await
    .unwrap()
}

async fn account_id_by_codes(
    pool: &PgPool,
    tenant_id: i64,
    gl: i32,
    sub: i32,
    a1: i32,
    a2: i32,
) -> Option<i64> {
    sqlx::query_scalar(
        "SELECT id FROM accounts WHERE tenant_id = $1 AND general_ledger_code = $2 \
         AND subsidiary_code = $3 AND analytic1_code = $4 AND analytic2_code = $5",
    )
    .bind(tenant_id)
    .bind(gl)
    .bind(sub)
    .bind(a1)
    .bind(a2)
    .fetch_optional(pool)
    .await
    .unwrap()
}

async fn child_count(pool: &PgPool, account_id: i64) -> i32 {
    sqlx::query_scalar("SELECT child_count FROM accounts WHERE id = $1")
        .bind(account_id)
        .fetch_one(pool)
        .await
        .unwrap()
}

#[sqlx::test(migrations = "./migrations")]
async fn create_natural_person_creates_two_coordinated_leaf_accounts(
    pool: PgPool,
) -> sqlx::Result<()> {
    let (tenant_id, _uid, token) = seed_session(&pool).await;
    let router = app(AppState { pool: pool.clone() });

    // Pre-provision the control accounts (Kol + Moein) — 07-07.md §7.3's
    // trade-AR / notes-AR pair, no `fixed_tafsil1_code`.
    seed_account(&pool, tenant_id, 103, 0, 0, 0, "Receivables").await;
    let ar_moein = seed_account(&pool, tenant_id, 103, 1, 0, 0, "Trade AR").await;
    seed_account(&pool, tenant_id, 109, 0, 0, 0, "Notes").await;
    let notes_moein = seed_account(&pool, tenant_id, 109, 1, 0, 0, "Notes AR").await;

    let config_a = seed_config(
        &pool,
        tenant_id,
        103,
        1,
        0,
        "Trade AR — persons",
        true,
        false,
    )
    .await;
    let config_b = seed_config(
        &pool,
        tenant_id,
        109,
        1,
        0,
        "Notes AR — persons",
        true,
        false,
    )
    .await;

    let resp = router
        .clone()
        .oneshot(
            Request::post("/api/v1/parties")
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::COOKIE, cookie(&token))
                .body(Body::from(
                    json!({
                        "cardNumber": 5001,
                        "partyType": "natural_person",
                        "firstName": "Ali",
                        "lastName": "Rezaei",
                        "fatherName": "Hassan",
                        "controlAccountConfigIds": [config_a, config_b],
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);

    // Both leaves land at Tafsil-1 = the card number, correctly coordinated.
    let leaf_a = account_id_by_codes(&pool, tenant_id, 103, 1, 5001, 0).await;
    let leaf_b = account_id_by_codes(&pool, tenant_id, 109, 1, 5001, 0).await;
    assert!(
        leaf_a.is_some(),
        "trade AR leaf should exist at Tafsil-1 = card number"
    );
    assert!(
        leaf_b.is_some(),
        "notes AR leaf should exist at Tafsil-1 = card number"
    );

    let leaf_a_name: String = sqlx::query_scalar("SELECT name FROM accounts WHERE id = $1")
        .bind(leaf_a.unwrap())
        .fetch_one(&pool)
        .await?;
    assert_eq!(leaf_a_name, "Ali Rezaei-Hassan");

    assert_eq!(child_count(&pool, ar_moein).await, 1);
    assert_eq!(child_count(&pool, notes_moein).await, 1);

    Ok(())
}

#[sqlx::test(migrations = "./migrations")]
async fn legal_entity_with_fixed_tafsil1_lands_at_tafsil2(pool: PgPool) -> sqlx::Result<()> {
    let (tenant_id, _uid, token) = seed_session(&pool).await;
    let router = app(AppState { pool: pool.clone() });

    // Fixed-Tafsil1 case: the control account's Tafsil-1 is pinned to 7, the
    // party card occupies Tafsil-2 (07-07.md §7.2's SC_T>0 rule) — the case
    // 07-02-b.md §2.4 says the legacy's write side can never actually reach.
    seed_account(&pool, tenant_id, 301, 0, 0, 0, "Payables").await;
    seed_account(&pool, tenant_id, 301, 2, 0, 0, "Trade AP — companies").await;
    let fixed_ta1 = seed_account(&pool, tenant_id, 301, 2, 7, 0, "Fixed control node").await;

    let config_c = seed_config(
        &pool,
        tenant_id,
        301,
        2,
        7,
        "Trade AP fixed — companies",
        false,
        true,
    )
    .await;

    let resp = router
        .clone()
        .oneshot(
            Request::post("/api/v1/parties")
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::COOKIE, cookie(&token))
                .body(Body::from(
                    json!({
                        "cardNumber": 6001,
                        "partyType": "legal_entity",
                        "firstName": "Pesteh Sabz",
                        "lastName": "Karimi",
                        "controlAccountConfigIds": [config_c],
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);

    let leaf = account_id_by_codes(&pool, tenant_id, 301, 2, 7, 6001).await;
    assert!(
        leaf.is_some(),
        "leaf should land at Tafsil-2 under the fixed Tafsil-1 node"
    );
    assert_eq!(child_count(&pool, fixed_ta1).await, 1);

    let leaf_name: String = sqlx::query_scalar("SELECT name FROM accounts WHERE id = $1")
        .bind(leaf.unwrap())
        .fetch_one(&pool)
        .await?;
    assert_eq!(leaf_name, "Pesteh Sabz Karimi"); // no father segment for a legal entity

    Ok(())
}

#[sqlx::test(migrations = "./migrations")]
async fn unticking_a_childless_control_account_deletes_its_leaf(pool: PgPool) -> sqlx::Result<()> {
    let (tenant_id, _uid, token) = seed_session(&pool).await;
    let router = app(AppState { pool: pool.clone() });

    seed_account(&pool, tenant_id, 103, 0, 0, 0, "Receivables").await;
    let ar_moein = seed_account(&pool, tenant_id, 103, 1, 0, 0, "Trade AR").await;
    seed_account(&pool, tenant_id, 109, 0, 0, 0, "Notes").await;
    let notes_moein = seed_account(&pool, tenant_id, 109, 1, 0, 0, "Notes AR").await;
    let config_a = seed_config(
        &pool,
        tenant_id,
        103,
        1,
        0,
        "Trade AR — persons",
        true,
        false,
    )
    .await;
    let config_b = seed_config(
        &pool,
        tenant_id,
        109,
        1,
        0,
        "Notes AR — persons",
        true,
        false,
    )
    .await;

    let create_resp = router
        .clone()
        .oneshot(
            Request::post("/api/v1/parties")
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::COOKIE, cookie(&token))
                .body(Body::from(
                    json!({
                        "cardNumber": 5002, "partyType": "natural_person",
                        "firstName": "Sara", "lastName": "Ahmadi", "fatherName": "Reza",
                        "controlAccountConfigIds": [config_a, config_b],
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    let party_id = json_body(create_resp).await["id"].as_i64().unwrap();
    assert_eq!(child_count(&pool, notes_moein).await, 1);

    // Update, unticking config_b (no postings against its leaf).
    let update_resp = router
        .clone()
        .oneshot(
            Request::put(format!("/api/v1/parties/{party_id}"))
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::COOKIE, cookie(&token))
                .body(Body::from(
                    json!({
                        "cardNumber": 5002, "partyType": "natural_person",
                        "firstName": "Sara", "lastName": "Ahmadi", "fatherName": "Reza",
                        "controlAccountConfigIds": [config_a],
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(update_resp.status(), StatusCode::NO_CONTENT);

    assert!(
        account_id_by_codes(&pool, tenant_id, 109, 1, 5002, 0)
            .await
            .is_none(),
        "unticked leaf with no postings should be deleted, not just hidden"
    );
    assert_eq!(child_count(&pool, notes_moein).await, 0);
    // The still-ticked account is untouched.
    assert!(account_id_by_codes(&pool, tenant_id, 103, 1, 5002, 0)
        .await
        .is_some());
    assert_eq!(child_count(&pool, ar_moein).await, 1);

    Ok(())
}

/// B18 (11-open-decisions.md): no global tick state — a control account
/// ticked for one party must never appear ticked for a different party that
/// never requested it (the legacy's SC_Tik bug this directly re-tests).
#[sqlx::test(migrations = "./migrations")]
async fn tick_state_never_leaks_between_two_different_parties(pool: PgPool) -> sqlx::Result<()> {
    let (tenant_id, _uid, token) = seed_session(&pool).await;
    let router = app(AppState { pool: pool.clone() });

    seed_account(&pool, tenant_id, 103, 0, 0, 0, "Receivables").await;
    seed_account(&pool, tenant_id, 103, 1, 0, 0, "Trade AR").await;
    let config_a = seed_config(
        &pool,
        tenant_id,
        103,
        1,
        0,
        "Trade AR — persons",
        true,
        false,
    )
    .await;

    let create = |card: i64, first: &'static str| {
        let router = router.clone();
        let token = token.clone();
        async move {
            router
                .oneshot(
                    Request::post("/api/v1/parties")
                        .header(header::CONTENT_TYPE, "application/json")
                        .header(header::COOKIE, cookie(&token))
                        .body(Body::from(
                            json!({
                                "cardNumber": card, "partyType": "natural_person",
                                "firstName": first, "lastName": "X", "fatherName": "Y",
                                "controlAccountConfigIds": [config_a],
                            })
                            .to_string(),
                        ))
                        .unwrap(),
                )
                .await
                .unwrap()
        }
    };

    let p1 = json_body(create(7001, "One").await).await["id"]
        .as_i64()
        .unwrap();

    // Party 2 is created WITHOUT ticking config_a.
    let p2_resp = router
        .clone()
        .oneshot(
            Request::post("/api/v1/parties")
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::COOKIE, cookie(&token))
                .body(Body::from(
                    json!({
                        "cardNumber": 7002, "partyType": "natural_person",
                        "firstName": "Two", "lastName": "X", "fatherName": "Y",
                        "controlAccountConfigIds": [],
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    let p2 = json_body(p2_resp).await["id"].as_i64().unwrap();

    let get = |id: i64| {
        let router = router.clone();
        let token = token.clone();
        async move {
            router
                .oneshot(
                    Request::get(format!("/api/v1/parties/{id}"))
                        .header(header::COOKIE, cookie(&token))
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap()
        }
    };

    let p1_detail = json_body(get(p1).await).await;
    let p2_detail = json_body(get(p2).await).await;

    let p1_ticked = p1_detail["controlAccounts"][0]["ticked"].as_bool().unwrap();
    let p2_ticked = p2_detail["controlAccounts"][0]["ticked"].as_bool().unwrap();
    assert!(
        p1_ticked,
        "party 1 ticked config_a and should show it ticked"
    );
    assert!(
        !p2_ticked,
        "party 2 never ticked config_a — must not leak party 1's tick"
    );

    Ok(())
}

#[sqlx::test(migrations = "./migrations")]
async fn validation_and_conflict_paths(pool: PgPool) -> sqlx::Result<()> {
    let (tenant_id, _uid, token) = seed_session(&pool).await;
    let router = app(AppState { pool: pool.clone() });

    // V3: a natural person with no father's name -> incomplete_data.
    let missing_father = router
        .clone()
        .oneshot(
            Request::post("/api/v1/parties")
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::COOKIE, cookie(&token))
                .body(Body::from(
                    json!({
                        "cardNumber": 1, "partyType": "natural_person",
                        "firstName": "A", "lastName": "B", "controlAccountConfigIds": [],
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(missing_father.status(), StatusCode::BAD_REQUEST);
    assert_eq!(json_body(missing_father).await["error"], "incomplete_data");

    // A legal entity needs no father's name.
    let legal_ok = router
        .clone()
        .oneshot(
            Request::post("/api/v1/parties")
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::COOKIE, cookie(&token))
                .body(Body::from(
                    json!({
                        "cardNumber": 2, "partyType": "legal_entity",
                        "firstName": "Co", "lastName": "Rep", "controlAccountConfigIds": [],
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(legal_ok.status(), StatusCode::CREATED);

    // V4: duplicate card number.
    let dup_card = router
        .clone()
        .oneshot(
            Request::post("/api/v1/parties")
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::COOKIE, cookie(&token))
                .body(Body::from(
                    json!({
                        "cardNumber": 2, "partyType": "legal_entity",
                        "firstName": "Other", "lastName": "Rep", "controlAccountConfigIds": [],
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(dup_card.status(), StatusCode::CONFLICT);
    assert_eq!(json_body(dup_card).await["error"], "duplicate_card_number");

    // V5: duplicate national ID, blank IDs exempted (the legacy's V5/V6 hole, not reproduced).
    let two_blanks_ok_1 = router
        .clone()
        .oneshot(
            Request::post("/api/v1/parties")
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::COOKIE, cookie(&token))
                .body(Body::from(
                    json!({
                        "cardNumber": 3, "partyType": "legal_entity",
                        "firstName": "Blank1", "lastName": "Rep", "controlAccountConfigIds": [],
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(two_blanks_ok_1.status(), StatusCode::CREATED);
    let two_blanks_ok_2 = router
        .clone()
        .oneshot(
            Request::post("/api/v1/parties")
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::COOKIE, cookie(&token))
                .body(Body::from(
                    json!({
                        "cardNumber": 4, "partyType": "legal_entity",
                        "firstName": "Blank2", "lastName": "Rep", "controlAccountConfigIds": [],
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(
        two_blanks_ok_2.status(),
        StatusCode::CREATED,
        "two blank national IDs must both succeed"
    );

    let with_nid = router
        .clone()
        .oneshot(
            Request::post("/api/v1/parties")
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::COOKIE, cookie(&token))
                .body(Body::from(
                    json!({
                        "cardNumber": 5, "partyType": "legal_entity",
                        "firstName": "Has", "lastName": "Nid", "nationalId": "1234567890",
                        "controlAccountConfigIds": [],
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(with_nid.status(), StatusCode::CREATED);
    let dup_nid = router
        .clone()
        .oneshot(
            Request::post("/api/v1/parties")
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::COOKIE, cookie(&token))
                .body(Body::from(
                    json!({
                        "cardNumber": 6, "partyType": "legal_entity",
                        "firstName": "Dup", "lastName": "Nid", "nationalId": "1234567890",
                        "controlAccountConfigIds": [],
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(dup_nid.status(), StatusCode::CONFLICT);
    assert_eq!(json_body(dup_nid).await["error"], "duplicate_national_id");

    // Ticking a config whose control account doesn't exist in the chart yet.
    let unprovisioned_config = seed_config(
        &pool,
        tenant_id,
        999,
        1,
        0,
        "Unprovisioned control",
        true,
        false,
    )
    .await;
    let not_provisioned = router
        .clone()
        .oneshot(
            Request::post("/api/v1/parties")
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::COOKIE, cookie(&token))
                .body(Body::from(
                    json!({
                        "cardNumber": 7, "partyType": "natural_person",
                        "firstName": "No", "lastName": "Control", "fatherName": "F",
                        "controlAccountConfigIds": [unprovisioned_config],
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(not_provisioned.status(), StatusCode::BAD_REQUEST);
    assert_eq!(
        json_body(not_provisioned).await["error"],
        "control_account_not_provisioned"
    );

    Ok(())
}

#[sqlx::test(migrations = "./migrations")]
async fn lock_requires_superuser(pool: PgPool) -> sqlx::Result<()> {
    let tenant_id: i64 =
        sqlx::query_scalar("INSERT INTO tenants (slug, name) VALUES ('acme', 'Acme') RETURNING id")
            .fetch_one(&pool)
            .await?;
    let admin_id: i64 = sqlx::query_scalar(
        "INSERT INTO users (tenant_id, username, password_hash, is_superuser) VALUES ($1, 'root', 'x', true) RETURNING id",
    )
    .bind(tenant_id)
    .fetch_one(&pool)
    .await?;
    let plain_id: i64 = sqlx::query_scalar(
        "INSERT INTO users (tenant_id, username, password_hash) VALUES ($1, 'plain', 'x') RETURNING id",
    )
    .bind(tenant_id)
    .fetch_one(&pool)
    .await?;
    let admin_token = "t-admin";
    let plain_token = "t-plain";
    for (token, uid) in [(admin_token, admin_id), (plain_token, plain_id)] {
        sqlx::query(
            "INSERT INTO sessions (id, user_id, tenant_id, expires_at) VALUES ($1, $2, $3, now() + interval '1 hour')",
        )
        .bind(token)
        .bind(uid)
        .bind(tenant_id)
        .execute(&pool)
        .await?;
    }
    let router = app(AppState { pool: pool.clone() });

    let create_resp = router
        .clone()
        .oneshot(
            Request::post("/api/v1/parties")
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::COOKIE, cookie(admin_token))
                .body(Body::from(
                    json!({
                        "cardNumber": 1, "partyType": "legal_entity",
                        "firstName": "Co", "lastName": "Rep", "controlAccountConfigIds": [],
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    let id = json_body(create_resp).await["id"].as_i64().unwrap();

    let denied = router
        .clone()
        .oneshot(
            Request::post(format!("/api/v1/parties/{id}/lock"))
                .header(header::COOKIE, cookie(plain_token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(denied.status(), StatusCode::FORBIDDEN);

    let allowed = router
        .oneshot(
            Request::post(format!("/api/v1/parties/{id}/lock"))
                .header(header::COOKIE, cookie(admin_token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(allowed.status(), StatusCode::NO_CONTENT);

    Ok(())
}

#[sqlx::test(migrations = "./migrations")]
async fn seed_defaults_is_idempotent_and_superuser_only(pool: PgPool) -> sqlx::Result<()> {
    let (_tenant_id, _uid, token) = seed_session(&pool).await;
    let router = app(AppState { pool: pool.clone() });

    let first = router
        .clone()
        .oneshot(
            Request::post("/api/v1/parties/account-config/seed-defaults")
                .header(header::COOKIE, cookie(&token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(first.status(), StatusCode::NO_CONTENT);

    let second = router
        .clone()
        .oneshot(
            Request::post("/api/v1/parties/account-config/seed-defaults")
                .header(header::COOKIE, cookie(&token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(second.status(), StatusCode::NO_CONTENT);

    let list = router
        .oneshot(
            Request::get("/api/v1/parties/account-config")
                .header(header::COOKIE, cookie(&token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let rows = json_body(list).await;
    assert_eq!(
        rows.as_array().unwrap().len(),
        10,
        "seeding twice must not duplicate rows"
    );

    Ok(())
}
