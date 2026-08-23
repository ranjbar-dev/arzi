//! Step 1.3 (docs/phase-1-platform-and-auth.md §1.3): user & permission
//! administration — the replacement for the legacy `Admin.pas` grid. All
//! routes require `RequireSuperuser` (08-04.md §4.4's "Supervisor-only, no
//! Pass_Config id" bucket — never delegable via the permission matrix, same
//! as the legacy).
//!
//! No self-service "first login, set your password" flow exists: a fresh
//! account's `password_hash` is the unusable sentinel (auth::NO_PASSWORD_
//! SENTINEL) and login always rejects it, full stop (verified by step 1.2's
//! manual test #6). Getting a real password onto the account is this
//! module's `set_user_password` — an admin action, same real-world shape as
//! the legacy's `C_Pass` ("Change Password", Admin.pas:281-292) — rather
//! than a pre-auth token/email flow this phase has no infrastructure for.
//! Judgment call, recorded here per the doc's "first-login" phrasing not
//! quite fitting a system with no email/token delivery yet.

use super::{authz::RequireSuperuser, hash_password, internal_error, NO_PASSWORD_SENTINEL};
use crate::{audit, db, AppState};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/users", get(list_users).post(create_user))
        .route("/users/{id}/enable", post(enable_user))
        .route("/users/{id}/disable", post(disable_user))
        .route("/users/{id}/set-password", post(set_user_password))
        .route(
            "/users/{id}/permissions",
            get(get_user_permissions).put(replace_user_permissions),
        )
        .route("/permissions", get(list_permission_catalogue))
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PermissionRow {
    id: i32,
    code: String,
    label_fa: String,
}

/// Step 1.6's admin screen needs to know what's grantable — the seeded
/// catalogue from 1.1's migration (08-04.md §4.4), not duplicated as a
/// hand-maintained list in the frontend.
async fn list_permission_catalogue(
    State(state): State<AppState>,
    _admin: RequireSuperuser,
) -> Result<Json<Vec<PermissionRow>>, (StatusCode, Json<Value>)> {
    let rows: Vec<(i32, String, String)> =
        sqlx::query_as("SELECT id, code, label_fa FROM permissions ORDER BY id")
            .fetch_all(&state.pool)
            .await
            .map_err(|_| internal_error())?;
    Ok(Json(
        rows.into_iter()
            .map(|(id, code, label_fa)| PermissionRow { id, code, label_fa })
            .collect(),
    ))
}

async fn get_user_permissions(
    State(state): State<AppState>,
    admin: RequireSuperuser,
    Path(id): Path<i64>,
) -> Result<Json<Vec<i32>>, (StatusCode, Json<Value>)> {
    let mut tx = db::begin(&state.pool, admin.0.tenant_id)
        .await
        .map_err(|_| internal_error())?;
    let ids: Vec<i32> = sqlx::query_scalar(
        "SELECT permission_id FROM user_permissions WHERE user_id = $1 ORDER BY permission_id",
    )
    .bind(id)
    .fetch_all(&mut *tx)
    .await
    .map_err(|_| internal_error())?;
    tx.rollback().await.ok();
    Ok(Json(ids))
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct UserRow {
    id: i64,
    username: String,
    is_active: bool,
    is_superuser: bool,
    created_at: DateTime<Utc>,
}

async fn list_users(
    State(state): State<AppState>,
    admin: RequireSuperuser,
) -> Result<Json<Vec<UserRow>>, (StatusCode, Json<Value>)> {
    let mut tx = db::begin(&state.pool, admin.0.tenant_id)
        .await
        .map_err(|_| internal_error())?;

    let rows: Vec<(i64, String, bool, bool, DateTime<Utc>)> = sqlx::query_as(
        "SELECT id, username, is_active, is_superuser, created_at \
         FROM users WHERE tenant_id = $1 ORDER BY id",
    )
    .bind(admin.0.tenant_id)
    .fetch_all(&mut *tx)
    .await
    .map_err(|_| internal_error())?;
    tx.rollback().await.ok();

    let users = rows
        .into_iter()
        .map(
            |(id, username, is_active, is_superuser, created_at)| UserRow {
                id,
                username,
                is_active,
                is_superuser,
                created_at,
            },
        )
        .collect();
    Ok(Json(users))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateUserRequest {
    username: String,
}

async fn create_user(
    State(state): State<AppState>,
    admin: RequireSuperuser,
    Json(req): Json<CreateUserRequest>,
) -> Result<(StatusCode, Json<Value>), (StatusCode, Json<Value>)> {
    let mut tx = db::begin(&state.pool, admin.0.tenant_id)
        .await
        .map_err(|_| internal_error())?;

    // No usable password (08-04.md §4.3 fix — the legacy created a user
    // with an empty password that could log in immediately).
    let id: i64 = sqlx::query_scalar(
        "INSERT INTO users (tenant_id, username, password_hash, created_by) \
         VALUES ($1, $2, $3, $4) RETURNING id",
    )
    .bind(admin.0.tenant_id)
    .bind(&req.username)
    .bind(NO_PASSWORD_SENTINEL)
    .bind(admin.0.user_id)
    .fetch_one(&mut *tx)
    .await
    .map_err(conflict_or_internal)?;

    audit::record_mutation(
        &mut tx,
        admin.0.tenant_id,
        "users",
        id,
        "insert",
        Some(admin.0.user_id),
        None,
        Some(json!({ "username": req.username, "isActive": true, "isSuperuser": false })),
    )
    .await
    .map_err(|_| internal_error())?;

    tx.commit().await.map_err(|_| internal_error())?;
    Ok((StatusCode::CREATED, Json(json!({ "id": id }))))
}

fn conflict_or_internal(err: sqlx::Error) -> (StatusCode, Json<Value>) {
    if let sqlx::Error::Database(db_err) = &err {
        if db_err.code().as_deref() == Some("23505") {
            return (
                StatusCode::CONFLICT,
                Json(json!({ "error": "username_taken" })),
            );
        }
    }
    internal_error()
}

async fn enable_user(
    State(state): State<AppState>,
    admin: RequireSuperuser,
    Path(id): Path<i64>,
) -> Result<StatusCode, (StatusCode, Json<Value>)> {
    set_active(&state, &admin.0, id, true).await
}

async fn disable_user(
    State(state): State<AppState>,
    admin: RequireSuperuser,
    Path(id): Path<i64>,
) -> Result<StatusCode, (StatusCode, Json<Value>)> {
    set_active(&state, &admin.0, id, false).await?;
    revoke_all_sessions(&state, id).await
}

async fn set_active(
    state: &AppState,
    admin: &super::AuthUser,
    user_id: i64,
    active: bool,
) -> Result<StatusCode, (StatusCode, Json<Value>)> {
    let mut tx = db::begin(&state.pool, admin.tenant_id)
        .await
        .map_err(|_| internal_error())?;

    let previous: Option<bool> = sqlx::query_scalar("SELECT is_active FROM users WHERE id = $1")
        .bind(user_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|_| internal_error())?;
    let Some(previous) = previous else {
        return Err((
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "user_not_found" })),
        ));
    };

    sqlx::query("UPDATE users SET is_active = $1, updated_at = now() WHERE id = $2")
        .bind(active)
        .bind(user_id)
        .execute(&mut *tx)
        .await
        .map_err(|_| internal_error())?;

    audit::record_mutation(
        &mut tx,
        admin.tenant_id,
        "users",
        user_id,
        "update",
        Some(admin.user_id),
        Some(json!({ "isActive": previous })),
        Some(json!({ "isActive": active })),
    )
    .await
    .map_err(|_| internal_error())?;

    tx.commit().await.map_err(|_| internal_error())?;
    Ok(StatusCode::NO_CONTENT)
}

async fn revoke_all_sessions(
    state: &AppState,
    user_id: i64,
) -> Result<StatusCode, (StatusCode, Json<Value>)> {
    sqlx::query("UPDATE sessions SET revoked_at = now() WHERE user_id = $1 AND revoked_at IS NULL")
        .bind(user_id)
        .execute(&state.pool)
        .await
        .map_err(|_| internal_error())?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SetPasswordRequest {
    new_password: String,
}

/// Admin-driven initial password / reset — see the module doc comment for
/// why this replaces a self-service "first login" flow.
async fn set_user_password(
    State(state): State<AppState>,
    admin: RequireSuperuser,
    Path(id): Path<i64>,
    Json(req): Json<SetPasswordRequest>,
) -> Result<StatusCode, (StatusCode, Json<Value>)> {
    if req.new_password.len() < super::MIN_PASSWORD_LEN {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "password_too_short" })),
        ));
    }

    let hash = hash_password(&req.new_password);
    let mut tx = db::begin(&state.pool, admin.0.tenant_id)
        .await
        .map_err(|_| internal_error())?;

    let result =
        sqlx::query("UPDATE users SET password_hash = $1, updated_at = now() WHERE id = $2")
            .bind(&hash)
            .bind(id)
            .execute(&mut *tx)
            .await
            .map_err(|_| internal_error())?;
    if result.rows_affected() == 0 {
        return Err((
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "user_not_found" })),
        ));
    }

    // Never log the hash itself (old or new) — just that it changed, by whom.
    audit::record_auth_event(
        &mut tx,
        admin.0.tenant_id,
        "password_changed",
        Some(id),
        Some(admin.0.user_id),
        Some(json!({ "setBy": "admin" })),
    )
    .await
    .map_err(|_| internal_error())?;

    tx.commit().await.map_err(|_| internal_error())?;
    // The account's old password (if any) is now dead — kill any session
    // that was issued under it too.
    revoke_all_sessions(&state, id).await
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ReplacePermissionsRequest {
    permission_ids: Vec<i32>,
}

/// Transactional delete-then-reinsert — the legacy's `B_SaveClick`
/// (Admin.pas:192-214) did the same two statements *without* a transaction,
/// so a crash between them left a partial or empty grant set (08-04.md
/// §4.2). Wrapping both in one transaction is the actual fix.
async fn replace_user_permissions(
    State(state): State<AppState>,
    admin: RequireSuperuser,
    Path(id): Path<i64>,
    Json(req): Json<ReplacePermissionsRequest>,
) -> Result<StatusCode, (StatusCode, Json<Value>)> {
    let mut tx = db::begin(&state.pool, admin.0.tenant_id)
        .await
        .map_err(|_| internal_error())?;

    let before: Vec<i32> =
        sqlx::query_scalar("SELECT permission_id FROM user_permissions WHERE user_id = $1")
            .bind(id)
            .fetch_all(&mut *tx)
            .await
            .map_err(|_| internal_error())?;
    let before: std::collections::HashSet<i32> = before.into_iter().collect();
    let after: std::collections::HashSet<i32> = req.permission_ids.iter().copied().collect();

    sqlx::query("DELETE FROM user_permissions WHERE user_id = $1")
        .bind(id)
        .execute(&mut *tx)
        .await
        .map_err(|_| internal_error())?;

    for permission_id in &req.permission_ids {
        sqlx::query(
            "INSERT INTO user_permissions (tenant_id, user_id, permission_id, granted_by) \
             VALUES ($1, $2, $3, $4)",
        )
        .bind(admin.0.tenant_id)
        .bind(id)
        .bind(permission_id)
        .bind(admin.0.user_id)
        .execute(&mut *tx)
        .await
        .map_err(|_| internal_error())?;
    }

    // 08-04.md §4.2 fix: the legacy's delete-then-reinsert has no history at
    // all (08-05.md §5.3). Log what actually changed, not just the new set.
    let granted: Vec<i32> = after.difference(&before).copied().collect();
    let revoked: Vec<i32> = before.difference(&after).copied().collect();
    if !granted.is_empty() {
        audit::record_auth_event(
            &mut tx,
            admin.0.tenant_id,
            "permission_granted",
            Some(id),
            Some(admin.0.user_id),
            Some(json!({ "permissionIds": granted })),
        )
        .await
        .map_err(|_| internal_error())?;
    }
    if !revoked.is_empty() {
        audit::record_auth_event(
            &mut tx,
            admin.0.tenant_id,
            "permission_revoked",
            Some(id),
            Some(admin.0.user_id),
            Some(json!({ "permissionIds": revoked })),
        )
        .await
        .map_err(|_| internal_error())?;
    }

    tx.commit().await.map_err(|_| internal_error())?;
    Ok(StatusCode::NO_CONTENT)
}
