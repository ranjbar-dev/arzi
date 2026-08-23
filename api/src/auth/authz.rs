//! Step 1.3 (docs/phase-1-platform-and-auth.md §1.3): the authorization
//! mechanism. `specs/08-04-authorization.md` §4.2 is the "don't do this"
//! reference — the legacy check is presentation-only (assigns `.Enabled`/
//! `.Visible` on a VCL control), so any route reached another way, or the
//! shared SQL login directly, is unconstrained. Every extractor here runs
//! server-side, before the handler body, so there's no route where the
//! check can be skipped.

use super::AuthUser;
use crate::AppState;
use axum::{
    extract::{FromRef, FromRequestParts},
    http::{request::Parts, StatusCode},
    Json,
};
use serde_json::{json, Value};
use std::marker::PhantomData;

impl AuthUser {
    pub fn has_permission(&self, code: &str) -> bool {
        // is_superuser bypasses the check within this user's own tenant
        // only (08-04.md §4.1) — AuthUser itself is already scoped to one
        // tenant via the session, so there's no cross-tenant path here.
        self.is_superuser || self.permissions.contains(code)
    }
}

/// One zero-sized marker type per gated route, e.g.
/// `impl Perm for ViewAccountList { const CODE: &'static str = "account_list"; }`.
/// Wiring these to real routes is later phases' job (2.8, 4.x, 6.7) — this
/// step only builds the mechanism.
pub trait Perm {
    const CODE: &'static str;
}

/// Extractor: 403s unless the session's user has `P::CODE` (or is a
/// tenant-scoped superuser). Take this as a handler argument instead of
/// `AuthUser` to make a route permission-gated — enforced at the route
/// level via the type system, not a call the handler author can forget.
pub struct RequirePermission<P: Perm>(pub AuthUser, PhantomData<P>);

impl<S, P: Perm> FromRequestParts<S> for RequirePermission<P>
where
    AppState: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = StatusCode;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let auth = AuthUser::from_request_parts(parts, state).await?;
        if !auth.has_permission(P::CODE) {
            return Err(StatusCode::FORBIDDEN);
        }
        Ok(RequirePermission(auth, PhantomData))
    }
}

/// Step 6.7's runtime-check twin of `RequirePermission<P>` — for the
/// reporting routes (6.1-6.6), where the correct id sometimes depends on
/// the request itself (e.g. `ledgers.rs`'s merged Kol+Moein endpoint needs
/// EITHER of two legacy ids, which the type-level extractor can't express;
/// same reasoning `vouchers.rs` already used for its own runtime checks
/// since 2.8) — a JSON error body, not the extractor's bare `StatusCode`.
pub fn require(auth: &AuthUser, code: &str) -> Result<(), (StatusCode, Json<Value>)> {
    require_any(auth, &[code])
}

pub fn require_any(auth: &AuthUser, codes: &[&str]) -> Result<(), (StatusCode, Json<Value>)> {
    if codes.iter().any(|c| auth.has_permission(c)) {
        Ok(())
    } else {
        Err((
            StatusCode::FORBIDDEN,
            Json(json!({ "error": "insufficient_permission" })),
        ))
    }
}

/// Extractor for the legacy's "Supervisor-only (no Pass_Config id)" bucket
/// (08-04.md §4.4 closing table) — actions with no catalogue permission at
/// all, never delegable through the matrix. User/permission administration
/// itself (this step's `admin` module) is the only member so far.
pub struct RequireSuperuser(pub AuthUser);

impl<S> FromRequestParts<S> for RequireSuperuser
where
    AppState: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = StatusCode;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let auth = AuthUser::from_request_parts(parts, state).await?;
        if !auth.is_superuser {
            return Err(StatusCode::FORBIDDEN);
        }
        Ok(RequireSuperuser(auth))
    }
}

/// Step 7.3's own gate — deliberately NOT implied by `RequireSuperuser`. A tenant's superuser is
/// scoped to that tenant only (08-04.md §4.1); backups cover the whole shared Postgres instance
/// (every tenant's data, per A3's shared-database tenancy), so reusing `is_superuser` here would
/// let any tenant's admin trigger and download a dump containing every OTHER tenant's data too —
/// an explicit user decision (2026-08-23) to add a genuinely separate, tenant-independent role
/// flag instead of overloading the existing one.
pub struct RequirePlatformAdmin(pub AuthUser);

impl<S> FromRequestParts<S> for RequirePlatformAdmin
where
    AppState: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = StatusCode;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let auth = AuthUser::from_request_parts(parts, state).await?;
        if !auth.is_platform_admin {
            return Err(StatusCode::FORBIDDEN);
        }
        Ok(RequirePlatformAdmin(auth))
    }
}
