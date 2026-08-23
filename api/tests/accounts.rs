//! Automated version of step 2.1's manual test (docs/phase-2-accounting-
//! core.md §2.1): build a 4-level hierarchy with correct child_count,
//! reject a duplicate tuple, report leaf-only postability, rename, and
//! reject deleting a node with children.

use api::{app, AppState};
use axum::{
    body::Body,
    http::{header, Request, StatusCode},
};
use serde_json::Value;
use sqlx::PgPool;
use tower::ServiceExt;

async fn seed_session(pool: &PgPool) -> (i64, String) {
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
    (tenant_id, token)
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

async fn create(
    router: &axum::Router,
    token: &str,
    parent_id: Option<i64>,
    code: i32,
    name: &str,
) -> axum::response::Response {
    router
        .clone()
        .oneshot(
            Request::post("/api/v1/accounts")
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::COOKIE, cookie(token))
                .body(Body::from(
                    serde_json::json!({ "parentId": parent_id, "code": code, "name": name })
                        .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap()
}

#[sqlx::test(migrations = "./migrations")]
async fn build_hierarchy_with_correct_child_counts(pool: PgPool) -> sqlx::Result<()> {
    let (_tenant_id, token) = seed_session(&pool).await;
    let router = app(AppState { pool: pool.clone() });

    let kol = create(&router, &token, None, 1, "Assets").await;
    assert_eq!(kol.status(), StatusCode::CREATED);
    let kol_id = json_body(kol).await["id"].as_i64().unwrap();

    let moein = create(&router, &token, Some(kol_id), 11, "Cash").await;
    assert_eq!(moein.status(), StatusCode::CREATED);
    let moein_id = json_body(moein).await["id"].as_i64().unwrap();

    let taf1 = create(&router, &token, Some(moein_id), 1, "On Hand").await;
    assert_eq!(taf1.status(), StatusCode::CREATED);
    let taf1_id = json_body(taf1).await["id"].as_i64().unwrap();

    let taf2 = create(&router, &token, Some(taf1_id), 1, "Petty Cash").await;
    assert_eq!(taf2.status(), StatusCode::CREATED);
    let taf2_id = json_body(taf2).await["id"].as_i64().unwrap();

    for (id, expected_child_count) in [(kol_id, 1), (moein_id, 1), (taf1_id, 1), (taf2_id, 0)] {
        let resp = router
            .clone()
            .oneshot(
                Request::get(format!("/api/v1/accounts/{id}"))
                    .header(header::COOKIE, cookie(&token))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let body = json_body(resp).await;
        assert_eq!(body["childCount"], expected_child_count, "account {id}");
    }

    // Duplicate tuple -> rejected.
    let dup = create(&router, &token, None, 1, "Assets Again").await;
    assert_eq!(dup.status(), StatusCode::CONFLICT);

    // Moein has children -> not a leaf -> not postable.
    let moein_detail = router
        .clone()
        .oneshot(
            Request::get(format!("/api/v1/accounts/{moein_id}"))
                .header(header::COOKIE, cookie(&token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let body = json_body(moein_detail).await;
    assert!(
        body["childCount"].as_i64().unwrap() > 0,
        "moein should not be a leaf"
    );
    assert_eq!(body["codeLtr"], "1-11");
    assert_eq!(body["fullNamePath"], "Assets/Cash");

    // Rename, confirm display formats reflect it.
    let rename = router
        .clone()
        .oneshot(
            Request::put(format!("/api/v1/accounts/{taf2_id}/name"))
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::COOKIE, cookie(&token))
                .body(Body::from(r#"{"name":"Renamed Cash"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(rename.status(), StatusCode::NO_CONTENT);

    let taf2_detail = router
        .clone()
        .oneshot(
            Request::get(format!("/api/v1/accounts/{taf2_id}"))
                .header(header::COOKIE, cookie(&token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let body = json_body(taf2_detail).await;
    assert_eq!(body["lineName"], "1 Renamed Cash");
    assert_eq!(body["fullNamePath"], "Assets/Cash/On Hand/Renamed Cash");
    assert!(body["codeRtl"].as_str().unwrap().ends_with("Renamed Cash"));

    // Delete a node with children -> rejected.
    let delete_taf1 = router
        .clone()
        .oneshot(
            Request::delete(format!("/api/v1/accounts/{taf1_id}"))
                .header(header::COOKIE, cookie(&token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(delete_taf1.status(), StatusCode::BAD_REQUEST);

    // Delete the leaf -> succeeds, parent's child_count drops back to 0.
    let delete_taf2 = router
        .clone()
        .oneshot(
            Request::delete(format!("/api/v1/accounts/{taf2_id}"))
                .header(header::COOKIE, cookie(&token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(delete_taf2.status(), StatusCode::NO_CONTENT);

    let taf1_after: i32 = sqlx::query_scalar("SELECT child_count FROM accounts WHERE id = $1")
        .bind(taf1_id)
        .fetch_one(&pool)
        .await?;
    assert_eq!(taf1_after, 0);

    Ok(())
}

#[sqlx::test(migrations = "./migrations")]
async fn promote_and_demote_move_a_leaf_between_levels(pool: PgPool) -> sqlx::Result<()> {
    let (_tenant_id, token) = seed_session(&pool).await;
    let router = app(AppState { pool: pool.clone() });

    let kol = create(&router, &token, None, 1, "Assets").await;
    let kol_id = json_body(kol).await["id"].as_i64().unwrap();
    let moein = create(&router, &token, Some(kol_id), 11, "Cash").await;
    let moein_id = json_body(moein).await["id"].as_i64().unwrap();
    let taf1 = create(&router, &token, Some(moein_id), 1, "On Hand").await;
    let taf1_id = json_body(taf1).await["id"].as_i64().unwrap();
    let taf2 = create(&router, &token, Some(taf1_id), 5, "Petty Cash").await;
    let taf2_id = json_body(taf2).await["id"].as_i64().unwrap();

    // Promote the Tafsil2 leaf to Tafsil1 (docs/phase-2 §2.2 manual test #2).
    let promote = router
        .clone()
        .oneshot(
            Request::post(format!("/api/v1/accounts/{taf2_id}/promote"))
                .header(header::COOKIE, cookie(&token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(promote.status(), StatusCode::NO_CONTENT);

    let (gl, sub, a1, a2, level): (i32, i32, i32, i32, i16) = sqlx::query_as(
        "SELECT general_ledger_code, subsidiary_code, analytic1_code, analytic2_code, level \
         FROM accounts WHERE id = $1",
    )
    .bind(taf2_id)
    .fetch_one(&pool)
    .await?;
    assert_eq!((gl, sub, a1, a2, level), (1, 11, 5, 0, 3));

    // taf1 (now a sibling instead of a parent) lost the child it used to have.
    let taf1_children: i32 = sqlx::query_scalar("SELECT child_count FROM accounts WHERE id = $1")
        .bind(taf1_id)
        .fetch_one(&pool)
        .await?;
    assert_eq!(taf1_children, 0);
    // moein gained a Tafsil1 child.
    let moein_children: i32 = sqlx::query_scalar("SELECT child_count FROM accounts WHERE id = $1")
        .bind(moein_id)
        .fetch_one(&pool)
        .await?;
    assert_eq!(moein_children, 2);

    // Demote it back under taf1.
    let demote = router
        .clone()
        .oneshot(
            Request::post(format!("/api/v1/accounts/{taf2_id}/demote"))
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::COOKIE, cookie(&token))
                .body(Body::from(
                    serde_json::json!({ "parentId": taf1_id }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(demote.status(), StatusCode::NO_CONTENT);

    let (gl, sub, a1, a2, level): (i32, i32, i32, i32, i16) = sqlx::query_as(
        "SELECT general_ledger_code, subsidiary_code, analytic1_code, analytic2_code, level \
         FROM accounts WHERE id = $1",
    )
    .bind(taf2_id)
    .fetch_one(&pool)
    .await?;
    assert_eq!((gl, sub, a1, a2, level), (1, 11, 1, 5, 4));

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
    let account = create(&router, admin_token, None, 1, "Assets").await;
    let id = json_body(account).await["id"].as_i64().unwrap();

    let denied = router
        .clone()
        .oneshot(
            Request::post(format!("/api/v1/accounts/{id}/lock"))
                .header(header::COOKIE, cookie(plain_token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(denied.status(), StatusCode::FORBIDDEN);

    let allowed = router
        .oneshot(
            Request::post(format!("/api/v1/accounts/{id}/lock"))
                .header(header::COOKIE, cookie(admin_token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(allowed.status(), StatusCode::NO_CONTENT);

    Ok(())
}
