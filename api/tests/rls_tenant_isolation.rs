//! Automated version of step 1.1's manual test (docs/phase-1-platform-and-auth.md
//! §1.1): proves cross-tenant reads/writes are blocked by Postgres RLS itself,
//! not merely by application-layer filtering.
//!
//! Needs a real, reachable Postgres — `#[sqlx::test]` creates and migrates a
//! throwaway database per test using DATABASE_URL's connection (must be a
//! role with CREATEDB, e.g. the compose `POSTGRES_USER` from `.env`), then
//! `SET ROLE` switches the session to `APP_DB_USER` (created by
//! `db/init/01-app-role.sh`, cluster-wide) so the RLS policies — which exempt
//! superusers — are actually exercised. Run with:
//!   DATABASE_URL=postgres://arzi:change_me@localhost:5432/arzi cargo test -p api

use sqlx::{Acquire, PgPool};

#[sqlx::test(migrations = "./migrations")]
async fn cross_tenant_rows_are_invisible_and_unwritable(pool: PgPool) -> sqlx::Result<()> {
    let app_role = std::env::var("APP_DB_USER").unwrap_or_else(|_| "arzi_app".to_string());

    // sqlx::test's ephemeral per-test database doesn't inherit the
    // ALTER DEFAULT PRIVILEGES grant that db/init/01-app-role.sh set up on
    // the real `arzi` database (default privileges are per-database) — grant
    // it here instead, as the owner role the test pool is already connected
    // as. Production/dev still rely on the init script; this is test-only.
    sqlx::query(&format!(
        "GRANT SELECT, INSERT, UPDATE, DELETE ON ALL TABLES IN SCHEMA public TO {app_role}"
    ))
    .execute(&pool)
    .await?;

    // Two tenants, seeded as the (superuser) owner role — RLS doesn't apply yet.
    let tenant_1: i64 = sqlx::query_scalar(
        "INSERT INTO tenants (slug, name) VALUES ('tenant-1', 'Tenant One') RETURNING id",
    )
    .fetch_one(&pool)
    .await?;
    let tenant_2: i64 = sqlx::query_scalar(
        "INSERT INTO tenants (slug, name) VALUES ('tenant-2', 'Tenant Two') RETURNING id",
    )
    .fetch_one(&pool)
    .await?;

    let mut conn = pool.acquire().await?;

    // From here on, act as the API's actual runtime role — non-superuser,
    // NOBYPASSRLS — exactly like a real request would.
    sqlx::query(&format!("SET ROLE {app_role}"))
        .execute(&mut *conn)
        .await?;

    let mut tx = conn.begin().await?;
    sqlx::query("SELECT set_config('app.tenant_id', $1, true)")
        .bind(tenant_1.to_string())
        .execute(&mut *tx)
        .await?;

    sqlx::query("INSERT INTO users (tenant_id, username, password_hash) VALUES ($1, 'alice', 'x')")
        .bind(tenant_1)
        .execute(&mut *tx)
        .await?;

    // Insert for tenant_2 while app.tenant_id is still tenant_1 — WITH CHECK
    // must reject it.
    let cross_tenant_insert = sqlx::query(
        "INSERT INTO users (tenant_id, username, password_hash) VALUES ($1, 'mallory', 'x')",
    )
    .bind(tenant_2)
    .execute(&mut *tx)
    .await;
    assert!(
        cross_tenant_insert.is_err(),
        "RLS WITH CHECK should reject an insert for a different tenant_id"
    );

    tx.rollback().await?;

    // Seed one user per tenant as the owner role (bypasses RLS), then read
    // back as the app role scoped to tenant_1 only.
    sqlx::query("INSERT INTO users (tenant_id, username, password_hash) VALUES ($1, 'alice', 'x')")
        .bind(tenant_1)
        .execute(&pool)
        .await?;
    sqlx::query("INSERT INTO users (tenant_id, username, password_hash) VALUES ($1, 'bob', 'x')")
        .bind(tenant_2)
        .execute(&pool)
        .await?;

    let mut tx = conn.begin().await?;
    sqlx::query("SELECT set_config('app.tenant_id', $1, true)")
        .bind(tenant_1.to_string())
        .execute(&mut *tx)
        .await?;

    let visible_usernames: Vec<String> = sqlx::query_scalar("SELECT username FROM users")
        .fetch_all(&mut *tx)
        .await?;
    assert_eq!(
        visible_usernames,
        vec!["alice".to_string()],
        "SELECT with app.tenant_id = tenant_1 must return only tenant_1's rows"
    );

    tx.rollback().await?;
    Ok(())
}

#[sqlx::test(migrations = "./migrations")]
async fn app_role_cannot_bypass_rls(pool: PgPool) -> sqlx::Result<()> {
    let app_role = std::env::var("APP_DB_USER").unwrap_or_else(|_| "arzi_app".to_string());

    let bypasses_rls: bool =
        sqlx::query_scalar("SELECT rolbypassrls FROM pg_roles WHERE rolname = $1")
            .bind(&app_role)
            .fetch_one(&pool)
            .await?;
    assert!(!bypasses_rls, "API role must not have BYPASSRLS");

    let is_superuser: bool = sqlx::query_scalar("SELECT rolsuper FROM pg_roles WHERE rolname = $1")
        .bind(&app_role)
        .fetch_one(&pool)
        .await?;
    assert!(!is_superuser, "API role must not be a superuser either");

    Ok(())
}
