//! Step 1.4 (docs/phase-1-platform-and-auth.md §1.4): the one helper every
//! mutating handler calls, inside the same transaction as the mutation it's
//! logging, to write an `audit_log` row. `specs/08-05-audit-trail-change-log.
//! md` §5.3 is the "what the legacy has zero of" list this replaces — deletes,
//! permission grants, logins, password changes, none of it recorded today.
//!
//! Deliberately a plain Rust function, not a Postgres trigger per table
//! (`specs/10-target-architecture.md` §2.6) — the call site is visible in the
//! same Rust source as the mutation, not hidden in a DDL file.

use serde_json::Value;
use sqlx::{Postgres, Transaction};

/// A table-row mutation: `table_name`/`record_id` identify the row,
/// `old_values`/`new_values` are a partial JSON snapshot of just the fields
/// that matter (never include `password_hash` or any other secret).
pub async fn record_mutation(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: i64,
    table_name: &str,
    record_id: impl std::fmt::Display,
    action: &str,
    changed_by: Option<i64>,
    old_values: Option<Value>,
    new_values: Option<Value>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO audit_log \
         (tenant_id, table_name, record_id, action, changed_by, old_values, new_values) \
         VALUES ($1, $2, $3, $4, $5, $6, $7)",
    )
    .bind(tenant_id)
    .bind(table_name)
    .bind(record_id.to_string())
    .bind(action)
    .bind(changed_by)
    .bind(old_values)
    .bind(new_values)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

/// A security event with no single business-table row to diff — always
/// `table_name = 'auth_events'` (module doc on the migration). `record_id` is
/// the affected user's id when one resolved (`None` for e.g. a login attempt
/// against a username that doesn't exist).
pub async fn record_auth_event(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: i64,
    action: &str,
    record_id: Option<i64>,
    changed_by: Option<i64>,
    details: Option<Value>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO audit_log \
         (tenant_id, table_name, record_id, action, changed_by, new_values) \
         VALUES ($1, 'auth_events', $2, $3, $4, $5)",
    )
    .bind(tenant_id)
    .bind(record_id.map(|id| id.to_string()))
    .bind(action)
    .bind(changed_by)
    .bind(details)
    .execute(&mut **tx)
    .await?;
    Ok(())
}
