use sqlx::{PgPool, Postgres, Transaction};

/// Opens a transaction scoped to one tenant: issues `SET LOCAL app.tenant_id
/// = $1` as its first statement, which every RLS policy from step 1.1 reads
/// via `current_setting('app.tenant_id')`. `tenant_id` must come from the
/// authenticated session server-side — never from a request header or body
/// field (specs/10-target-architecture.md §2.4). Called from `auth::` once
/// tenant_id is known (from the tenant-slug lookup on login, or from the
/// session row on every later authenticated request).
pub async fn begin(
    pool: &PgPool,
    tenant_id: i64,
) -> Result<Transaction<'_, Postgres>, sqlx::Error> {
    let mut tx = pool.begin().await?;
    sqlx::query("SELECT set_config('app.tenant_id', $1, true)")
        .bind(tenant_id.to_string())
        .execute(&mut *tx)
        .await?;
    Ok(tx)
}
