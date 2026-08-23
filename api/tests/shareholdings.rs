//! Automated version of step 3.3's manual test (docs/phase-3-parties.md
//! §3.3): the API-level wiring around the pure-function arithmetic already
//! unit-tested in api/src/shareholdings.rs — create three holdings, list
//! shows correct `ownershipPercentage`, and the profit-distribution endpoint
//! reproduces the worked example end to end through real HTTP + Postgres.

use api::{app, AppState};
use axum::{
    body::Body,
    http::{header, Request, StatusCode},
};
use serde_json::{json, Value};
use sqlx::PgPool;
use tower::ServiceExt;

async fn seed(pool: &PgPool) -> (i64, i64, String) {
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

async fn seed_party(pool: &PgPool, tenant_id: i64, card: i32, name: &str) -> i64 {
    sqlx::query_scalar(
        "INSERT INTO parties (tenant_id, card_number, party_type, first_name, last_name, father_name) \
         VALUES ($1, $2, 'natural_person', $3, 'Holder', 'F') RETURNING id",
    )
    .bind(tenant_id)
    .bind(card)
    .bind(name)
    .fetch_one(pool)
    .await
    .unwrap()
}

async fn create_holding(
    router: &axum::Router,
    token: &str,
    party_id: i64,
    share_count: i64,
    join_date: &str,
    exit_date: Option<&str>,
) -> i64 {
    let resp = router
        .clone()
        .oneshot(
            Request::post("/api/v1/shareholdings")
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::COOKIE, cookie(token))
                .body(Body::from(
                    json!({
                        "partyId": party_id, "shareCount": share_count,
                        "joinDate": join_date, "exitDate": exit_date,
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    json_body(resp).await["id"].as_i64().unwrap()
}

#[sqlx::test(migrations = "./migrations")]
async fn list_shows_correct_percentages_and_distribution_matches_worked_example(pool: PgPool) -> sqlx::Result<()> {
    let (tenant_id, _uid, token) = seed(&pool).await;
    let router = app(AppState { pool: pool.clone() });

    let p1 = seed_party(&pool, tenant_id, 1, "A").await;
    let p2 = seed_party(&pool, tenant_id, 2, "B").await;
    let p3 = seed_party(&pool, tenant_id, 3, "C").await;

    create_holding(&router, &token, p1, 500, "2020-01-01", None).await;
    create_holding(&router, &token, p2, 300, "2020-01-01", None).await;
    create_holding(&router, &token, p3, 200, "2020-01-01", None).await;

    let list = router
        .clone()
        .oneshot(
            Request::get("/api/v1/shareholdings")
                .header(header::COOKIE, cookie(&token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let rows = json_body(list).await;
    let rows = rows.as_array().unwrap();
    assert_eq!(rows.len(), 3);
    let pct_for = |party_id: i64| {
        rows.iter().find(|r| r["partyId"] == party_id).unwrap()["ownershipPercentage"].as_f64().unwrap()
    };
    assert!((pct_for(p1) - 50.0).abs() < 1e-9);
    assert!((pct_for(p2) - 30.0).abs() < 1e-9);
    assert!((pct_for(p3) - 20.0).abs() < 1e-9);

    let fiscal_year_id: i64 = sqlx::query_scalar(
        "INSERT INTO fiscal_years (tenant_id, year, start_date, end_date) \
         VALUES ($1, 1399, '2020-03-20', '2021-03-20') RETURNING id",
    )
    .bind(tenant_id)
    .fetch_one(&pool)
    .await?;

    let dist = router
        .oneshot(
            Request::post("/api/v1/shareholdings/profit-distribution")
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::COOKIE, cookie(&token))
                .body(Body::from(
                    json!({ "fiscalYearId": fiscal_year_id, "profitAmount": 100_000_000i64 }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(dist.status(), StatusCode::OK);
    let allocations = json_body(dist).await;
    let allocations = allocations.as_array().unwrap();
    assert_eq!(allocations.len(), 3);
    let alloc_for = |party_id: i64| {
        allocations.iter().find(|r| r["partyId"] == party_id).unwrap()["allocation"].as_i64().unwrap()
    };
    assert_eq!(alloc_for(p1), 50_000_000);
    assert_eq!(alloc_for(p2), 30_000_000);
    assert_eq!(alloc_for(p3), 20_000_000);

    Ok(())
}

#[sqlx::test(migrations = "./migrations")]
async fn exited_shareholder_excluded_and_remaining_reproportioned(pool: PgPool) -> sqlx::Result<()> {
    let (tenant_id, _uid, token) = seed(&pool).await;
    let router = app(AppState { pool: pool.clone() });

    let p1 = seed_party(&pool, tenant_id, 1, "A").await;
    let p2 = seed_party(&pool, tenant_id, 2, "B").await;
    let p3 = seed_party(&pool, tenant_id, 3, "C").await;

    // p1 exits well before the fiscal year in question.
    create_holding(&router, &token, p1, 500, "2015-01-01", Some("2016-06-01")).await;
    create_holding(&router, &token, p2, 300, "2015-01-01", None).await;
    create_holding(&router, &token, p3, 200, "2015-01-01", None).await;

    let fiscal_year_id: i64 = sqlx::query_scalar(
        "INSERT INTO fiscal_years (tenant_id, year, start_date, end_date) \
         VALUES ($1, 1400, '2021-03-21', '2022-03-20') RETURNING id",
    )
    .bind(tenant_id)
    .fetch_one(&pool)
    .await?;

    let dist = router
        .oneshot(
            Request::post("/api/v1/shareholdings/profit-distribution")
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::COOKIE, cookie(&token))
                .body(Body::from(
                    json!({ "fiscalYearId": fiscal_year_id, "profitAmount": 100_000_000i64 }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(dist.status(), StatusCode::OK);
    let allocations = json_body(dist).await;
    let allocations = allocations.as_array().unwrap();
    // p1 excluded entirely, p2/p3 reproportioned over 500 total (300:200).
    assert_eq!(allocations.len(), 2);
    let alloc_for = |party_id: i64| {
        allocations.iter().find(|r| r["partyId"] == party_id).unwrap()["allocation"].as_i64().unwrap()
    };
    assert_eq!(alloc_for(p2), 60_000_000);
    assert_eq!(alloc_for(p3), 40_000_000);

    Ok(())
}
