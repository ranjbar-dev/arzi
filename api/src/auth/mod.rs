//! Step 1.2 (docs/phase-1-platform-and-auth.md §1.2): login/logout/change-
//! password, Argon2id hashing, server-side sessions. Deliberately nothing
//! like `specs/08-03-authentication.md` — that doc exists only to describe
//! what NOT to port (plaintext passwords, enumerable user list, no lockout).
//!
//! Step 1.3 (docs/phase-1-platform-and-auth.md §1.3) adds the authorization
//! layer on top: `authz` (has_permission, the `RequirePermission`/
//! `RequireSuperuser` extractors) and `admin` (the user/permission
//! management endpoints those extractors gate).

pub mod admin;
pub mod authz;

pub use authz::{Perm, RequirePermission, RequireSuperuser};

use crate::{audit, db, AppState};
use argon2::{
    password_hash::{
        rand_core::{OsRng, RngCore},
        PasswordHash, PasswordHasher, PasswordVerifier, SaltString,
    },
    Argon2,
};
use axum::{
    extract::{FromRef, FromRequestParts, State},
    http::{request::Parts, StatusCode},
    routing::post,
    Json, Router,
};
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use chrono::{DateTime, Duration, Utc};
use serde::Deserialize;
use serde_json::{json, Value};
use sqlx::PgPool;
use std::collections::{HashMap, HashSet};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration as StdDuration, Instant};

const SESSION_COOKIE: &str = "arzi_session";
const SESSION_TTL_HOURS: i64 = 12;
const MIN_PASSWORD_LEN: usize = 8;
const RATE_LIMIT_WINDOW: StdDuration = StdDuration::from_secs(300);
const RATE_LIMIT_MAX_ATTEMPTS: u32 = 5;

/// New users (created by the 1.3 admin flow) get this as `password_hash`.
/// Not a valid PHC hash string, so `PasswordHash::new` always errors and
/// login always fails — closes the legacy hole where a fresh account had an
/// empty password and could log in immediately (08-04.md §4.3).
pub const NO_PASSWORD_SENTINEL: &str = "!";

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/login", post(login))
        .route("/logout", post(logout))
        .route("/change-password", post(change_password))
}

// ---- password hashing --------------------------------------------------

pub fn hash_password(password: &str) -> String {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .expect("argon2 hashing failed")
        .to_string()
}

fn verify_password(stored_hash: &str, candidate: &str) -> bool {
    match PasswordHash::new(stored_hash) {
        Ok(parsed) => Argon2::default()
            .verify_password(candidate.as_bytes(), &parsed)
            .is_ok(),
        Err(_) => false, // malformed/sentinel hash never verifies
    }
}

fn generate_session_token() -> String {
    let mut bytes = [0u8; 32];
    OsRng.fill_bytes(&mut bytes);
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

// ---- rate limiting (per tenant-slug/username pair, in-process) ---------
//
// ponytail: single-process in-memory fixed window. Fine for one API
// instance; if this ever runs behind multiple replicas, move the counter to
// Postgres/Redis so it's shared.

type RateKey = (String, String);

fn rate_limit_store() -> &'static Mutex<HashMap<RateKey, (u32, Instant)>> {
    static STORE: OnceLock<Mutex<HashMap<RateKey, (u32, Instant)>>> = OnceLock::new();
    STORE.get_or_init(|| Mutex::new(HashMap::new()))
}

fn is_rate_limited(key: &RateKey) -> bool {
    let store = rate_limit_store().lock().unwrap();
    matches!(
        store.get(key),
        Some((count, first)) if first.elapsed() < RATE_LIMIT_WINDOW && *count >= RATE_LIMIT_MAX_ATTEMPTS
    )
}

fn record_failure(key: RateKey) {
    let mut store = rate_limit_store().lock().unwrap();
    let entry = store.entry(key).or_insert((0, Instant::now()));
    if entry.1.elapsed() >= RATE_LIMIT_WINDOW {
        *entry = (0, Instant::now());
    }
    entry.0 += 1;
}

fn clear_rate_limit(key: &RateKey) {
    rate_limit_store().lock().unwrap().remove(key);
}

// ---- login ---------------------------------------------------------------

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct LoginRequest {
    tenant_slug: String,
    username: String,
    password: String,
}

enum LoginError {
    Invalid,
    Internal,
}

async fn login(
    State(state): State<AppState>,
    jar: CookieJar,
    Json(req): Json<LoginRequest>,
) -> Result<(CookieJar, Json<Value>), (StatusCode, Json<Value>)> {
    let key = (req.tenant_slug.clone(), req.username.clone());
    if is_rate_limited(&key) {
        return Err((
            StatusCode::TOO_MANY_REQUESTS,
            Json(json!({ "error": "too_many_attempts" })),
        ));
    }

    match attempt_login(&state.pool, &req).await {
        Ok((token, _expires_at)) => {
            clear_rate_limit(&key);
            let cookie = session_cookie(token);
            Ok((jar.add(cookie), Json(json!({ "status": "ok" }))))
        }
        Err(LoginError::Invalid) => {
            record_failure(key);
            Err(generic_auth_failure())
        }
        Err(LoginError::Internal) => Err(internal_error()),
    }
}

/// Resolve tenant from `tenantSlug` first, then look up `username` scoped to
/// that tenant only — never a cross-tenant username search
/// (`08-03.md` §3.3's comment on `PassWord`). The explicit `tenant_id`
/// filter is belt-and-suspenders alongside the RLS-scoped transaction: this
/// query must be correct even if RLS is ever misconfigured or bypassed.
async fn attempt_login(
    pool: &PgPool,
    req: &LoginRequest,
) -> Result<(String, DateTime<Utc>), LoginError> {
    let tenant_id: Option<i64> =
        sqlx::query_scalar("SELECT id FROM tenants WHERE slug = $1 AND is_active")
            .bind(&req.tenant_slug)
            .fetch_optional(pool)
            .await
            .map_err(|_| LoginError::Internal)?;
    let tenant_id = tenant_id.ok_or(LoginError::Invalid)?;

    let mut tx = db::begin(pool, tenant_id)
        .await
        .map_err(|_| LoginError::Internal)?;

    let user: Option<(i64, String, bool)> = sqlx::query_as(
        "SELECT id, password_hash, is_active FROM users WHERE tenant_id = $1 AND username = $2",
    )
    .bind(tenant_id)
    .bind(&req.username)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|_| LoginError::Internal)?;
    let Some((user_id, password_hash, is_active)) = user else {
        tx.rollback().await.map_err(|_| LoginError::Internal)?;
        log_login_failed(pool, tenant_id, None, &req.username).await;
        return Err(LoginError::Invalid);
    };

    tx.rollback().await.map_err(|_| LoginError::Internal)?; // read-only lookup

    // Generic failure for wrong tenant/username/password/disabled alike —
    // do not let any of these branches distinguish which one failed
    // (08-03.md §3.3 steps 3-4 is exactly the enumerable behaviour to kill).
    if !is_active || !verify_password(&password_hash, &req.password) {
        log_login_failed(pool, tenant_id, Some(user_id), &req.username).await;
        return Err(LoginError::Invalid);
    }

    let token = generate_session_token();
    let expires_at = Utc::now() + Duration::hours(SESSION_TTL_HOURS);
    sqlx::query(
        "INSERT INTO sessions (id, user_id, tenant_id, expires_at) VALUES ($1, $2, $3, $4)",
    )
    .bind(&token)
    .bind(user_id)
    .bind(tenant_id)
    .bind(expires_at)
    .execute(pool)
    .await
    .map_err(|_| LoginError::Internal)?;

    if let Ok(mut tx) = db::begin(pool, tenant_id).await {
        let _ = audit::record_auth_event(
            &mut tx,
            tenant_id,
            "login_succeeded",
            Some(user_id),
            Some(user_id),
            None,
        )
        .await;
        let _ = tx.commit().await;
    }

    Ok((token, expires_at))
}

/// Best-effort: a failure to *log* a failed login must never itself surface
/// as a login error (08-04.md §4.2's "presentation-only" mistake was in the
/// other direction — this is the audit path being non-load-bearing for auth
/// correctness, on purpose).
async fn log_login_failed(pool: &PgPool, tenant_id: i64, user_id: Option<i64>, username: &str) {
    if let Ok(mut tx) = db::begin(pool, tenant_id).await {
        let _ = audit::record_auth_event(
            &mut tx,
            tenant_id,
            "login_failed",
            user_id,
            None,
            Some(json!({ "username": username })),
        )
        .await;
        let _ = tx.commit().await;
    }
}

/// `SESSION_COOKIE_SECURE` (default `true`): browsers silently refuse to
/// store a `Secure` cookie over plain HTTP, which would break login outright
/// on every origin in this stack until step 7.4 puts TLS in front of it —
/// `.env`/`docker-compose.yml` set this to `false` for local/dev/compose use.
fn session_cookie_secure() -> bool {
    std::env::var("SESSION_COOKIE_SECURE")
        .map(|v| v != "false" && v != "0")
        .unwrap_or(true)
}

fn session_cookie(token: String) -> Cookie<'static> {
    Cookie::build((SESSION_COOKIE, token))
        .http_only(true)
        .secure(session_cookie_secure())
        .same_site(SameSite::Lax)
        .path("/")
        .max_age(time::Duration::hours(SESSION_TTL_HOURS))
        .build()
}

fn generic_auth_failure() -> (StatusCode, Json<Value>) {
    (
        StatusCode::UNAUTHORIZED,
        Json(json!({ "error": "invalid_credentials" })),
    )
}

fn internal_error() -> (StatusCode, Json<Value>) {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(json!({ "error": "internal_error" })),
    )
}

// ---- logout ---------------------------------------------------------------

async fn logout(
    State(state): State<AppState>,
    jar: CookieJar,
) -> Result<(CookieJar, StatusCode), StatusCode> {
    let Some(cookie) = jar.get(SESSION_COOKIE) else {
        return Err(StatusCode::BAD_REQUEST);
    };
    let token = cookie.value().to_string();

    sqlx::query("UPDATE sessions SET revoked_at = now() WHERE id = $1 AND revoked_at IS NULL")
        .bind(&token)
        .execute(&state.pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let cleared = Cookie::build((SESSION_COOKIE, ""))
        .path("/")
        .max_age(time::Duration::ZERO)
        .build();
    Ok((jar.add(cleared), StatusCode::NO_CONTENT))
}

// ---- change password --------------------------------------------------

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ChangePasswordRequest {
    current_password: String,
    new_password: String,
}

async fn change_password(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<ChangePasswordRequest>,
) -> Result<StatusCode, (StatusCode, Json<Value>)> {
    if req.new_password.len() < MIN_PASSWORD_LEN {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "password_too_short" })),
        ));
    }

    let mut tx = db::begin(&state.pool, auth.tenant_id)
        .await
        .map_err(|_| internal_error())?;

    let current_hash: Option<String> =
        sqlx::query_scalar("SELECT password_hash FROM users WHERE id = $1")
            .bind(auth.user_id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(|_| internal_error())?;
    let current_hash = current_hash.ok_or_else(internal_error)?;

    // Exact match required, like the legacy (08-03.md §3.5 check #1) — but
    // hashed, so "exact match" is just verify_password against the real hash.
    if !verify_password(&current_hash, &req.current_password) {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "current_password_incorrect" })),
        ));
    }

    let new_hash = hash_password(&req.new_password);
    sqlx::query("UPDATE users SET password_hash = $1, updated_at = now() WHERE id = $2")
        .bind(&new_hash)
        .bind(auth.user_id)
        .execute(&mut *tx)
        .await
        .map_err(|_| internal_error())?;

    audit::record_auth_event(
        &mut tx,
        auth.tenant_id,
        "password_changed",
        Some(auth.user_id),
        Some(auth.user_id),
        Some(json!({ "setBy": "self" })),
    )
    .await
    .map_err(|_| internal_error())?;

    tx.commit().await.map_err(|_| internal_error())?;

    // Invalidate every other session on this account.
    sqlx::query(
        "UPDATE sessions SET revoked_at = now() WHERE user_id = $1 AND id <> $2 AND revoked_at IS NULL",
    )
    .bind(auth.user_id)
    .bind(&auth.session_id)
    .execute(&state.pool)
    .await
    .map_err(|_| internal_error())?;

    Ok(StatusCode::NO_CONTENT)
}

// ---- session extractor --------------------------------------------------

/// Loads and validates the session named by the request's cookie, plus (step
/// 1.3) the user's authorization state — once per request, not once per
/// permission check like the legacy's `IsEnabel` sweep (08-04.md §4.2). Any
/// handler taking this (or `RequirePermission`/`RequireSuperuser`, which
/// wrap it) as an argument is a protected endpoint — axum runs the check
/// before the handler body, so there's no route where it's possible to
/// forget it.
pub struct AuthUser {
    pub session_id: String,
    pub user_id: i64,
    pub tenant_id: i64,
    pub is_superuser: bool,
    pub is_platform_admin: bool,
    pub permissions: HashSet<String>,
}

impl<S> FromRequestParts<S> for AuthUser
where
    AppState: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = StatusCode;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let state = AppState::from_ref(state);
        let jar = CookieJar::from_headers(&parts.headers);
        let token = jar
            .get(SESSION_COOKIE)
            .map(|c| c.value().to_string())
            .ok_or(StatusCode::UNAUTHORIZED)?;

        let row: Option<(i64, i64, DateTime<Utc>, Option<DateTime<Utc>>)> = sqlx::query_as(
            "SELECT user_id, tenant_id, expires_at, revoked_at FROM sessions WHERE id = $1",
        )
        .bind(&token)
        .fetch_optional(&state.pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        let (user_id, tenant_id, expires_at, revoked_at) = row.ok_or(StatusCode::UNAUTHORIZED)?;

        if revoked_at.is_some() || expires_at < Utc::now() {
            return Err(StatusCode::UNAUTHORIZED);
        }

        let (is_superuser, is_platform_admin, permissions) =
            load_authz(&state.pool, tenant_id, user_id)
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        Ok(AuthUser {
            session_id: token,
            user_id,
            tenant_id,
            is_superuser,
            is_platform_admin,
            permissions,
        })
    }
}

/// `user_permissions` joined to `permissions`, resolved to codes — the
/// `authz::has_permission` build bullet's data source, loaded once here
/// rather than queried per check.
async fn load_authz(
    pool: &PgPool,
    tenant_id: i64,
    user_id: i64,
) -> Result<(bool, bool, HashSet<String>), sqlx::Error> {
    let mut tx = db::begin(pool, tenant_id).await?;

    let (is_superuser, is_platform_admin): (bool, bool) =
        sqlx::query_as("SELECT is_superuser, is_platform_admin FROM users WHERE id = $1")
            .bind(user_id)
            .fetch_one(&mut *tx)
            .await?;

    let codes: Vec<String> = sqlx::query_scalar(
        "SELECT p.code FROM user_permissions up \
         JOIN permissions p ON p.id = up.permission_id \
         WHERE up.user_id = $1",
    )
    .bind(user_id)
    .fetch_all(&mut *tx)
    .await?;

    tx.rollback().await?; // read-only lookup

    Ok((is_superuser, is_platform_admin, codes.into_iter().collect()))
}
