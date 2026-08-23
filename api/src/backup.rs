//! Step 7.3 (docs/phase-7-hardening-and-cutover.md §7.3 / specs/08-platform-and-security/08-08-
//! backup-restore-new-company-import.md): real backup/restore. The legacy had backup but **no
//! restore at all** (§8.2 — "no unit reads an `.ABS` archive back") and ran its one auto-backup
//! unconditionally from `Reload` on every login, gated only by a process-lifetime flag (§8.1) — a
//! background job here instead, never smuggled into a request handler users are waiting on.
//!
//! **Authorization**: gated by `RequirePlatformAdmin`, not `RequireSuperuser` — see
//! `api/migrations/0021_backups.sql`'s header comment and `auth/authz.rs`'s `RequirePlatformAdmin`
//! doc comment for why a tenant-scoped superuser is the wrong gate for an instance-wide dump.
//!
//! **Where the dump lives**: `pg_dump`/`pg_restore` run as real subprocesses (`tokio::process`)
//! against `DATABASE_URL` (the owner role — a whole-instance dump needs unrestricted read access,
//! which the app's RLS-bound role deliberately does not have). Files land under `BACKUP_DIR`
//! (default `/backups`), a Docker-managed named volume mounted only into the `api` container —
//! durable across container recreation, and never a path the *client* (browser) can write to,
//! which is the actual legacy defect (§8.1: "client-side `FileExists` check is meaningless for a
//! remote server").
//!
//! **Restore is deliberately NOT an HTTP endpoint.** Restoring a `pg_restore` dump into the live,
//! shared, multi-tenant database while the app is running would drop every tenant's connections and
//! overwrite every tenant's current data from one point-in-time snapshot — a whole-instance
//! disaster-recovery operation, not a per-tenant self-service action, and not something that should
//! be one authenticated HTTP request away from happening by mistake. `scripts/restore-backup.sh`
//! is the real, tested restore path (this step's Build bullet) — a plain `pg_restore` wrapper run by
//! whoever has infra access, exercised against a scratch database as the step's own manual test.

use crate::auth::authz::RequirePlatformAdmin;
use crate::AppState;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post, put},
    Json, Router,
};
use serde::Deserialize;
use serde_json::{json, Value};
use sqlx::PgPool;
use std::path::PathBuf;

fn backup_dir() -> PathBuf {
    PathBuf::from(std::env::var("BACKUP_DIR").unwrap_or_else(|_| "/backups".to_string()))
}

fn retention_count() -> i64 {
    std::env::var("BACKUP_RETENTION_COUNT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(10)
}

fn internal_error(msg: impl std::fmt::Display) -> (StatusCode, Json<Value>) {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(json!({ "error": msg.to_string() })),
    )
}

/// Runs one `pg_dump`, records it in `backups`, and applies retention. Shared by the on-demand
/// HTTP handler and the scheduled background task (`spawn_scheduled_backups` below) — one
/// implementation, two callers, per this module's own "not smuggled into a request" framing: the
/// scheduled caller runs on a `tokio::time::interval`, never inside a login or any other user
/// request.
pub async fn run_backup(
    pool: &PgPool,
    trigger: &str,
    created_by: Option<i64>,
) -> Result<i64, String> {
    let dir = backup_dir();
    tokio::fs::create_dir_all(&dir)
        .await
        .map_err(|e| format!("cannot create backup dir: {e}"))?;

    let filename = format!(
        "arzi-{}.dump",
        chrono::Utc::now().format("%Y%m%dT%H%M%S%.3fZ")
    );
    let path = dir.join(&filename);

    let id: i64 = sqlx::query_scalar(
        "INSERT INTO backups (filename, status, trigger, created_by) VALUES ($1, 'running', $2, $3) RETURNING id",
    )
    .bind(&filename)
    .bind(trigger)
    .bind(created_by)
    .fetch_one(pool)
    .await
    .map_err(|e| format!("cannot record backup start: {e}"))?;

    let database_url =
        std::env::var("DATABASE_URL").map_err(|_| "DATABASE_URL not set".to_string())?;
    let output = tokio::process::Command::new("pg_dump")
        .arg(&database_url)
        .arg("-Fc") // custom format: compressed, restorable with pg_restore, matches scripts/restore-backup.sh
        .arg("-f")
        .arg(&path)
        .output()
        .await;

    match output {
        Ok(out) if out.status.success() => {
            let size = tokio::fs::metadata(&path)
                .await
                .map(|m| m.len() as i64)
                .unwrap_or(0);
            sqlx::query(
                "UPDATE backups SET status = 'completed', size_bytes = $1, completed_at = now() WHERE id = $2",
            )
            .bind(size)
            .bind(id)
            .execute(pool)
            .await
            .map_err(|e| format!("cannot record backup completion: {e}"))?;
            apply_retention(pool, &dir).await;
            Ok(id)
        }
        Ok(out) => {
            let stderr = String::from_utf8_lossy(&out.stderr).to_string();
            sqlx::query("UPDATE backups SET status = 'failed', error_message = $1, completed_at = now() WHERE id = $2")
                .bind(&stderr)
                .bind(id)
                .execute(pool)
                .await
                .ok();
            Err(stderr)
        }
        Err(e) => {
            let msg = format!("failed to spawn pg_dump: {e}");
            sqlx::query("UPDATE backups SET status = 'failed', error_message = $1, completed_at = now() WHERE id = $2")
                .bind(&msg)
                .bind(id)
                .execute(pool)
                .await
                .ok();
            Err(msg)
        }
    }
}

/// Keeps the newest `BACKUP_RETENTION_COUNT` completed backups; deletes the rest (row + file) —
/// best-effort, logged not propagated, since a retention hiccup should never fail the backup that
/// just succeeded.
async fn apply_retention(pool: &PgPool, dir: &std::path::Path) {
    let keep = retention_count();
    let stale: Vec<(i64, String)> = sqlx::query_as(
        "SELECT id, filename FROM backups WHERE status = 'completed' \
         ORDER BY started_at DESC OFFSET $1",
    )
    .bind(keep)
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    for (id, filename) in stale {
        let _ = tokio::fs::remove_file(dir.join(&filename)).await;
        let _ = sqlx::query("DELETE FROM backups WHERE id = $1")
            .bind(id)
            .execute(pool)
            .await;
    }
}

/// The background job (Build bullet: "runs as a real background job, not smuggled into every
/// login"). `interval_secs` is caller-supplied (main.rs reads `BACKUP_INTERVAL_SECS`, defaulting to
/// a real day in production) so tests never have to wait on it.
pub fn spawn_scheduled_backups(pool: PgPool, interval_secs: u64) {
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(std::time::Duration::from_secs(interval_secs));
        ticker.tick().await; // first tick fires immediately; skip it, don't dump at boot
        loop {
            ticker.tick().await;
            if let Err(e) = run_backup(&pool, "scheduled", None).await {
                eprintln!("scheduled backup failed: {e}");
            }
        }
    });
}

async fn create_backup(
    State(state): State<AppState>,
    RequirePlatformAdmin(auth): RequirePlatformAdmin,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let id = run_backup(&state.pool, "manual", Some(auth.user_id))
        .await
        .map_err(internal_error)?;
    Ok(Json(json!({ "id": id })))
}

type BackupRow = (
    i64,
    String,
    String,
    Option<i64>,
    Option<String>,
    String,
    chrono::DateTime<chrono::Utc>,
    Option<chrono::DateTime<chrono::Utc>>,
);

async fn list_backups(
    State(state): State<AppState>,
    RequirePlatformAdmin(_auth): RequirePlatformAdmin,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let rows: Vec<BackupRow> =
        sqlx::query_as(
            "SELECT id, filename, status::text, size_bytes, error_message, trigger, started_at, completed_at \
             FROM backups ORDER BY started_at DESC",
        )
        .fetch_all(&state.pool)
        .await
        .map_err(internal_error)?;

    let items: Vec<Value> = rows
        .into_iter()
        .map(
            |(
                id,
                filename,
                status,
                size_bytes,
                error_message,
                trigger,
                started_at,
                completed_at,
            )| {
                json!({
                    "id": id, "filename": filename, "status": status, "sizeBytes": size_bytes,
                    "errorMessage": error_message, "trigger": trigger,
                    "startedAt": started_at, "completedAt": completed_at,
                })
            },
        )
        .collect();
    Ok(Json(json!({ "items": items })))
}

async fn download_backup(
    State(state): State<AppState>,
    RequirePlatformAdmin(_auth): RequirePlatformAdmin,
    Path(id): Path<i64>,
) -> Result<axum::response::Response, (StatusCode, Json<Value>)> {
    let row: Option<(String, String)> =
        sqlx::query_as("SELECT filename, status::text FROM backups WHERE id = $1")
            .bind(id)
            .fetch_optional(&state.pool)
            .await
            .map_err(internal_error)?;
    let (filename, status) =
        row.ok_or((StatusCode::NOT_FOUND, Json(json!({ "error": "not_found" }))))?;
    if status != "completed" {
        return Err((
            StatusCode::CONFLICT,
            Json(json!({ "error": "backup_not_completed" })),
        ));
    }
    let bytes = tokio::fs::read(backup_dir().join(&filename))
        .await
        .map_err(|e| {
            (
                StatusCode::NOT_FOUND,
                Json(json!({ "error": format!("file missing on disk: {e}") })),
            )
        })?;
    Ok((
        [
            (
                axum::http::header::CONTENT_TYPE,
                "application/octet-stream".to_string(),
            ),
            (
                axum::http::header::CONTENT_DISPOSITION,
                format!("attachment; filename=\"{filename}\""),
            ),
        ],
        bytes,
    )
        .into_response())
}

#[derive(Deserialize)]
struct GrantPlatformAdminRequest {
    grant: bool,
}

/// Grants/revokes `is_platform_admin` on another user — scoped to the caller's OWN tenant (RLS on
/// `users` enforces this transparently, same as every other admin action in this codebase; there is
/// no cross-tenant user directory). The very FIRST platform admin, in any tenant, is a direct
/// database operation — same "no tenant-provisioning flow exists yet" gap already accepted since
/// 1.1/2.1/3.1/5.1 for other bootstrap concerns, not a new one this step introduces.
async fn set_platform_admin(
    State(state): State<AppState>,
    RequirePlatformAdmin(auth): RequirePlatformAdmin,
    Path(user_id): Path<i64>,
    Json(req): Json<GrantPlatformAdminRequest>,
) -> Result<StatusCode, (StatusCode, Json<Value>)> {
    let mut tx = crate::db::begin(&state.pool, auth.tenant_id)
        .await
        .map_err(internal_error)?;
    let result = sqlx::query("UPDATE users SET is_platform_admin = $1 WHERE id = $2")
        .bind(req.grant)
        .bind(user_id)
        .execute(&mut *tx)
        .await
        .map_err(internal_error)?;
    if result.rows_affected() == 0 {
        return Err((
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "user_not_found" })),
        ));
    }
    tx.commit().await.map_err(internal_error)?;
    Ok(StatusCode::NO_CONTENT)
}

/// Mounted at `/api/v1/platform` (lib.rs) — a deliberately separate namespace from
/// `/api/v1/admin` (tenant-scoped `RequireSuperuser`), so the URL itself signals "this is
/// instance-wide, not your tenant's admin panel."
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/backups", post(create_backup).get(list_backups))
        .route("/backups/{id}/download", get(download_backup))
        .route("/users/{user_id}/platform-admin", put(set_platform_admin))
}
