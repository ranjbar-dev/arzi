//! Step 2.1 (docs/phase-2-accounting-core.md §2.1): the 4-level chart-of-
//! accounts hierarchy (Kol/Moein/Tafsil1/Tafsil2), leaf-only posting.
//! specs/03-accounting-core/03-01-a/b-account-hierarchy-model.md and
//! 03-02-a/b-account-crud-rules.md are the behavioural ground truth this
//! module ports (and, for `promote`/`demote`, extends — see their doc
//! comments below, that pair has no legacy precedent at all).
//!
//! No `parent_id` column (02-13-a.md §13.6's proposal was NOT adopted — the
//! Build bullet keeps the legacy's positional-segments-only model and
//! derives `level` instead). Every "find the parent/children" query below is
//! therefore a segment-prefix match, not a join on a foreign key.
//!
//! Step 2.8 (docs/phase-2-accounting-core.md §2.8): every route below is now
//! gated by its catalogue permission id (03-13-permissions.md §13.2's
//! `sRollOutPanel1` group) via the static `RequirePermission<P>` extractor —
//! lock/unlock stay `RequireSuperuser` (no catalogue id, same bucket as
//! 1.3's admin routes). Resolves the legacy's 1102/1103 dead-code collision
//! (§13.2's note: `SNewu` enforced none of create/amend/delete at all, and
//! the one place that tried, `ListSarfaslu`, buggily assigned all three
//! results to the same control so only delete had any effect) — create,
//! amend (rename/recode/promote/demote), and delete are independently real
//! and independently enforced here. `promote`/`demote` have no catalogue id
//! of their own (no legacy precedent, see their own doc comment below) —
//! mapped to `amend_account` (1103) as the closest fit, a restructuring
//! action on an existing node.

use crate::{
    audit,
    auth::{
        authz::{Perm, RequirePermission, RequireSuperuser},
        AuthUser,
    },
    db, AppState,
};

/// Marker types for `RequirePermission<P>` — one per accounting-core chart-
/// of-accounts action, 03-13-permissions.md §13.2's `sRollOutPanel1` ids.
mod permcodes {
    use super::Perm;
    pub struct AccountList;
    impl Perm for AccountList {
        const CODE: &'static str = "account_list"; // 1101
    }
    pub struct CreateAccount;
    impl Perm for CreateAccount {
        const CODE: &'static str = "create_account"; // 1102
    }
    pub struct AmendAccount;
    impl Perm for AmendAccount {
        const CODE: &'static str = "amend_account"; // 1103
    }
    pub struct DeleteAccount;
    impl Perm for DeleteAccount {
        const CODE: &'static str = "delete_account"; // 1104
    }
}
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{get, post, put},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::{Postgres, Transaction};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(list_accounts).post(create_account))
        .route("/{id}", get(get_account).delete(delete_account))
        .route("/{id}/name", put(rename_account))
        .route("/{id}/code", put(recode_account))
        .route("/{id}/promote", post(promote_account))
        .route("/{id}/demote", post(demote_account))
        .route("/{id}/lock", post(lock_account))
        .route("/{id}/unlock", post(unlock_account))
}

fn internal_error() -> (StatusCode, Json<Value>) {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(json!({ "error": "internal_error" })),
    )
}
fn not_found() -> (StatusCode, Json<Value>) {
    (
        StatusCode::NOT_FOUND,
        Json(json!({ "error": "account_not_found" })),
    )
}
fn bad_request(error: &str) -> (StatusCode, Json<Value>) {
    (StatusCode::BAD_REQUEST, Json(json!({ "error": error })))
}
fn conflict_or_internal(err: sqlx::Error) -> (StatusCode, Json<Value>) {
    if let sqlx::Error::Database(db_err) = &err {
        if db_err.constraint() == Some("accounts_natural_key") {
            return (
                StatusCode::CONFLICT,
                Json(json!({ "error": "duplicate_code" })),
            );
        }
    }
    internal_error()
}

#[derive(sqlx::FromRow, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct AccountRecord {
    id: i64,
    general_ledger_code: i32,
    subsidiary_code: i32,
    analytic1_code: i32,
    analytic2_code: i32,
    name: String,
    child_count: i32,
    is_locked: bool,
    level: i16,
    is_active: bool,
    party_id: Option<i64>,
    address: Option<String>,
    phone: Option<String>,
    fax: Option<String>,
    registration_number: Option<String>,
    economic_code: Option<String>,
    postal_code: Option<String>,
    national_id: Option<String>,
}

const ACCOUNT_COLUMNS: &str = "id, general_ledger_code, subsidiary_code, analytic1_code, \
     analytic2_code, name, child_count, is_locked, level, is_active, party_id, address, phone, \
     fax, registration_number, economic_code, postal_code, national_id";

impl AccountRecord {
    fn code_at_level(&self, level: i16) -> i32 {
        match level {
            1 => self.general_ledger_code,
            2 => self.subsidiary_code,
            3 => self.analytic1_code,
            4 => self.analytic2_code,
            _ => unreachable!("account level is always 1..=4"),
        }
    }

    fn own_code(&self) -> i32 {
        self.code_at_level(self.level)
    }
}

async fn fetch_account(
    tx: &mut Transaction<'_, Postgres>,
    id: i64,
) -> Result<Option<AccountRecord>, sqlx::Error> {
    sqlx::query_as(&format!(
        "SELECT {ACCOUNT_COLUMNS} FROM accounts WHERE id = $1"
    ))
    .bind(id)
    .fetch_optional(&mut **tx)
    .await
}

/// The immediate parent of `account`, found by segment-prefix match (no
/// `parent_id` — see the module doc comment). `None` for a level-1 (Kol)
/// node, which has no parent.
async fn fetch_parent(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: i64,
    account: &AccountRecord,
) -> Result<Option<AccountRecord>, sqlx::Error> {
    let sql = match account.level {
        1 => return Ok(None),
        2 => format!(
            "SELECT {ACCOUNT_COLUMNS} FROM accounts \
             WHERE tenant_id = $1 AND general_ledger_code = $2 AND subsidiary_code = 0"
        ),
        3 => format!(
            "SELECT {ACCOUNT_COLUMNS} FROM accounts WHERE tenant_id = $1 \
             AND general_ledger_code = $2 AND subsidiary_code = $3 AND analytic1_code = 0"
        ),
        4 => format!(
            "SELECT {ACCOUNT_COLUMNS} FROM accounts WHERE tenant_id = $1 \
             AND general_ledger_code = $2 AND subsidiary_code = $3 AND analytic1_code = $4 \
             AND analytic2_code = 0"
        ),
        _ => unreachable!(),
    };
    let mut query = sqlx::query_as(&sql)
        .bind(tenant_id)
        .bind(account.general_ledger_code);
    if account.level >= 3 {
        query = query.bind(account.subsidiary_code);
    }
    if account.level >= 4 {
        query = query.bind(account.analytic1_code);
    }
    query.fetch_optional(&mut **tx).await
}

async fn adjust_child_count(
    tx: &mut Transaction<'_, Postgres>,
    account_id: i64,
    delta: i32,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE accounts SET child_count = child_count + $1, updated_at = now() WHERE id = $2",
    )
    .bind(delta)
    .bind(account_id)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

// ---- list / get ------------------------------------------------------

#[derive(Deserialize)]
struct ListQuery {
    parent: Option<i64>,
}

/// One endpoint replaces the legacy's four `Select_Kol`/`Select_Moein`/
/// `Select_Taf1`/`Select_Taf2` queries (02-11-c.md's DDL comment). No
/// `parent` => the top-level Kol nodes.
async fn list_accounts(
    State(state): State<AppState>,
    guard: RequirePermission<permcodes::AccountList>,
    Query(params): Query<ListQuery>,
) -> Result<Json<Vec<AccountRecord>>, (StatusCode, Json<Value>)> {
    let auth = guard.0;
    let mut tx = db::begin(&state.pool, auth.tenant_id)
        .await
        .map_err(|_| internal_error())?;

    let rows = match params.parent {
        None => sqlx::query_as(&format!(
            "SELECT {ACCOUNT_COLUMNS} FROM accounts \
             WHERE tenant_id = $1 AND subsidiary_code = 0 ORDER BY general_ledger_code"
        ))
        .bind(auth.tenant_id)
        .fetch_all(&mut *tx)
        .await
        .map_err(|_| internal_error())?,
        Some(parent_id) => {
            let Some(parent) = fetch_account(&mut tx, parent_id)
                .await
                .map_err(|_| internal_error())?
            else {
                return Err(not_found());
            };
            if parent.level == 4 {
                Vec::new() // Tafsil2 is always a leaf — no children possible.
            } else {
                let (sql, order_by) = match parent.level {
                    1 => (
                        "SELECT {cols} FROM accounts WHERE tenant_id = $1 \
                         AND general_ledger_code = $2 AND subsidiary_code > 0 AND analytic1_code = 0",
                        "subsidiary_code",
                    ),
                    2 => (
                        "SELECT {cols} FROM accounts WHERE tenant_id = $1 \
                         AND general_ledger_code = $2 AND subsidiary_code = $3 AND analytic1_code > 0 AND analytic2_code = 0",
                        "analytic1_code",
                    ),
                    3 => (
                        "SELECT {cols} FROM accounts WHERE tenant_id = $1 \
                         AND general_ledger_code = $2 AND subsidiary_code = $3 AND analytic1_code = $4 AND analytic2_code > 0",
                        "analytic2_code",
                    ),
                    _ => unreachable!(),
                };
                let sql = sql.replace("{cols}", ACCOUNT_COLUMNS) + &format!(" ORDER BY {order_by}");
                let mut query = sqlx::query_as(&sql)
                    .bind(auth.tenant_id)
                    .bind(parent.general_ledger_code);
                if parent.level >= 2 {
                    query = query.bind(parent.subsidiary_code);
                }
                if parent.level >= 3 {
                    query = query.bind(parent.analytic1_code);
                }
                query
                    .fetch_all(&mut *tx)
                    .await
                    .map_err(|_| internal_error())?
            }
        }
    };
    tx.rollback().await.ok();
    Ok(Json(rows))
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct AccountDetail {
    #[serde(flatten)]
    account: AccountRecord,
    code_ltr: String,
    code_rtl: String,
    full_name_path: String,
    line_name: String,
}

/// The four legacy display formats (03-01-a.md §1.4), computed from the
/// tuple — never stored. `ancestor_names` is ordered shallow-to-deep (Kol
/// first), i.e. every level strictly above `account.level`.
fn render_display(
    account: &AccountRecord,
    ancestor_names: &[String],
    widths: &[i32; 4],
) -> AccountDetail {
    let codes = [
        account.general_ledger_code,
        account.subsidiary_code,
        account.analytic1_code,
        account.analytic2_code,
    ];

    // (a) LTR, unpadded, dash-joined, trailing zero levels omitted.
    let code_ltr = codes
        .iter()
        .take_while(|&&c| c > 0)
        .map(|c| c.to_string())
        .collect::<Vec<_>>()
        .join("-");

    // (b) RTL, zero-padded to each level's configured width, deepest leftmost, name appended.
    let mut code_rtl = String::new();
    for (i, &c) in codes.iter().enumerate() {
        if c == 0 {
            break;
        }
        let width = widths[i];
        let padded = if width > 0 {
            format!("{c:0width$}", width = width as usize)
        } else {
            c.to_string()
        };
        code_rtl = if code_rtl.is_empty() {
            padded
        } else {
            format!("{padded}-{code_rtl}")
        };
    }
    code_rtl.push(' ');
    code_rtl.push_str(&account.name);

    // (d) '/'-joined NAMES down the path (ancestors, shallow to deep, then self).
    let mut full_name_path = ancestor_names.to_vec();
    full_name_path.push(account.name.clone());
    let full_name_path = full_name_path.join("/");

    // Last-level code + name (SanadEditU.pas:373's LineName).
    let line_name = format!("{} {}", account.own_code(), account.name);

    AccountDetail {
        account: account.clone(),
        code_ltr,
        code_rtl,
        full_name_path,
        line_name,
    }
}

/// Digit widths from 1.1's `account_code_format`, falling back to the
/// legacy's own hard-coded defaults (03-01-a.md §1.3) when a tenant hasn't
/// configured them (no tenant-provisioning flow seeds this table yet).
async fn code_widths(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: i64,
) -> Result<[i32; 4], sqlx::Error> {
    let rows: Vec<(i16, i16)> =
        sqlx::query_as("SELECT level, width FROM account_code_format WHERE tenant_id = $1")
            .bind(tenant_id)
            .fetch_all(&mut **tx)
            .await?;
    let mut widths = [3, 3, 4, 0]; // Kol=3, Moein=3, Tafsil1=4, Tafsil2 unconstrained (SNewu.pas:504,522).
    for (level, width) in rows {
        if (1..=4).contains(&level) {
            widths[(level - 1) as usize] = width as i32;
        }
    }
    Ok(widths)
}

async fn get_account(
    State(state): State<AppState>,
    guard: RequirePermission<permcodes::AccountList>,
    Path(id): Path<i64>,
) -> Result<Json<AccountDetail>, (StatusCode, Json<Value>)> {
    let auth = guard.0;
    let mut tx = db::begin(&state.pool, auth.tenant_id)
        .await
        .map_err(|_| internal_error())?;

    let Some(account) = fetch_account(&mut tx, id)
        .await
        .map_err(|_| internal_error())?
    else {
        return Err(not_found());
    };

    let mut ancestor_names = Vec::new();
    let mut current = account.clone();
    while let Some(parent) = fetch_parent(&mut tx, auth.tenant_id, &current)
        .await
        .map_err(|_| internal_error())?
    {
        ancestor_names.push(parent.name.clone());
        current = parent;
    }
    ancestor_names.reverse(); // was deep-to-shallow; display wants shallow-to-deep.

    let widths = code_widths(&mut tx, auth.tenant_id)
        .await
        .map_err(|_| internal_error())?;
    tx.rollback().await.ok();

    Ok(Json(render_display(&account, &ancestor_names, &widths)))
}

// ---- create ------------------------------------------------------------

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateAccountRequest {
    parent_id: Option<i64>,
    code: i32,
    name: String,
}

async fn create_account(
    State(state): State<AppState>,
    guard: RequirePermission<permcodes::CreateAccount>,
    Json(req): Json<CreateAccountRequest>,
) -> Result<(StatusCode, Json<Value>), (StatusCode, Json<Value>)> {
    let auth = guard.0;
    if req.code <= 0 {
        return Err(bad_request("invalid_code"));
    }
    if req.name.trim().is_empty() {
        return Err(bad_request("invalid_name")); // 03-02-a.md #2.1 check 1
    }

    let mut tx = db::begin(&state.pool, auth.tenant_id)
        .await
        .map_err(|_| internal_error())?;

    let (parent, level) = match req.parent_id {
        None => (None, 1i16),
        Some(pid) => {
            let Some(parent) = fetch_account(&mut tx, pid)
                .await
                .map_err(|_| internal_error())?
            else {
                return Err(not_found());
            };
            if parent.level == 4 {
                return Err(bad_request("max_depth_reached"));
            }
            let level = parent.level + 1;
            (Some(parent), level)
        }
    };

    let mut codes = [0i32; 4];
    if let Some(p) = &parent {
        codes[0] = p.general_ledger_code;
        codes[1] = p.subsidiary_code;
        codes[2] = p.analytic1_code;
    }
    codes[(level - 1) as usize] = req.code;

    let id: i64 = sqlx::query_scalar(
        "INSERT INTO accounts \
         (tenant_id, general_ledger_code, subsidiary_code, analytic1_code, analytic2_code, name, created_by) \
         VALUES ($1, $2, $3, $4, $5, $6, $7) RETURNING id",
    )
    .bind(auth.tenant_id)
    .bind(codes[0])
    .bind(codes[1])
    .bind(codes[2])
    .bind(codes[3])
    .bind(req.name.trim())
    .bind(auth.user_id)
    .fetch_one(&mut *tx)
    .await
    .map_err(conflict_or_internal)?;

    if let Some(p) = &parent {
        adjust_child_count(&mut tx, p.id, 1)
            .await
            .map_err(|_| internal_error())?;
    }

    audit::record_mutation(
        &mut tx,
        auth.tenant_id,
        "accounts",
        id,
        "insert",
        Some(auth.user_id),
        None,
        Some(json!({ "codes": codes, "name": req.name.trim() })),
    )
    .await
    .map_err(|_| internal_error())?;

    tx.commit().await.map_err(|_| internal_error())?;
    Ok((StatusCode::CREATED, Json(json!({ "id": id }))))
}

// ---- rename --------------------------------------------------------------

#[derive(Deserialize)]
struct RenameRequest {
    name: String,
}

/// No preconditions (03-02-a.md §2.2) — a name may change at any time, on
/// any node, even one with postings.
async fn rename_account(
    State(state): State<AppState>,
    guard: RequirePermission<permcodes::AmendAccount>,
    Path(id): Path<i64>,
    Json(req): Json<RenameRequest>,
) -> Result<StatusCode, (StatusCode, Json<Value>)> {
    let auth = guard.0;
    if req.name.trim().is_empty() {
        return Err(bad_request("invalid_name"));
    }
    let mut tx = db::begin(&state.pool, auth.tenant_id)
        .await
        .map_err(|_| internal_error())?;

    let Some(account) = fetch_account(&mut tx, id)
        .await
        .map_err(|_| internal_error())?
    else {
        return Err(not_found());
    };

    sqlx::query("UPDATE accounts SET name = $1, updated_at = now(), updated_by = $2 WHERE id = $3")
        .bind(req.name.trim())
        .bind(auth.user_id)
        .bind(id)
        .execute(&mut *tx)
        .await
        .map_err(|_| internal_error())?;

    audit::record_mutation(
        &mut tx,
        auth.tenant_id,
        "accounts",
        id,
        "update",
        Some(auth.user_id),
        Some(json!({ "name": account.name })),
        Some(json!({ "name": req.name.trim() })),
    )
    .await
    .map_err(|_| internal_error())?;

    tx.commit().await.map_err(|_| internal_error())?;
    Ok(StatusCode::NO_CONTENT)
}

// ---- recode --------------------------------------------------------------

#[derive(Deserialize)]
struct RecodeRequest {
    code: i32,
}

/// 03-02-a.md §2.3's validations, in order: leaf only (guard #2), new !=
/// old is a silent no-op (guard #4), duplicate rejected (guard #5). Guard #3
/// ("has postings") can't be implemented yet — `voucher_lines` doesn't exist
/// until step 2.3 — `has_postings` below is a stub always returning `false`,
/// wired up once that table lands (the manual test explicitly expects this
/// gap: "rejected once 2.3 exists; for now, confirm ... non-postable").
async fn recode_account(
    State(state): State<AppState>,
    guard: RequirePermission<permcodes::AmendAccount>,
    Path(id): Path<i64>,
    Json(req): Json<RecodeRequest>,
) -> Result<StatusCode, (StatusCode, Json<Value>)> {
    let auth = guard.0;
    if req.code <= 0 {
        return Err(bad_request("invalid_code"));
    }
    let mut tx = db::begin(&state.pool, auth.tenant_id)
        .await
        .map_err(|_| internal_error())?;

    let Some(account) = fetch_account(&mut tx, id)
        .await
        .map_err(|_| internal_error())?
    else {
        return Err(not_found());
    };
    if account.child_count > 0 {
        return Err(bad_request("has_children"));
    }
    let old_code = account.own_code();
    if req.code == old_code {
        tx.rollback().await.ok();
        return Ok(StatusCode::NO_CONTENT); // silent no-op, matches SNewu.pas:264
    }

    let column = match account.level {
        1 => "general_ledger_code",
        2 => "subsidiary_code",
        3 => "analytic1_code",
        4 => "analytic2_code",
        _ => unreachable!(),
    };
    sqlx::query(&format!(
        "UPDATE accounts SET {column} = $1, updated_at = now(), updated_by = $2 WHERE id = $3"
    ))
    .bind(req.code)
    .bind(auth.user_id)
    .bind(id)
    .execute(&mut *tx)
    .await
    .map_err(conflict_or_internal)?;

    audit::record_mutation(
        &mut tx,
        auth.tenant_id,
        "accounts",
        id,
        "update",
        Some(auth.user_id),
        Some(json!({ "code": old_code })),
        Some(json!({ "code": req.code })),
    )
    .await
    .map_err(|_| internal_error())?;

    tx.commit().await.map_err(|_| internal_error())?;
    Ok(StatusCode::NO_CONTENT)
}

// ---- promote / demote ----------------------------------------------------
//
// No legacy precedent (03-14-open-questions.md / 03-02-a/b describe no such
// operation) — docs/phase-2-accounting-core.md §2.2's manual test ("Promote
// a Tafsil2 node to Tafsil1") is the only spec for the intended behaviour.
// Judgment call, recorded here: promoting/demoting a node reuses its own
// current deepest code as the code at the new level, shifting it one level
// shallower/deeper in the tree. Both are leaf-only (same "SNO > 0" guard as
// recode) since a non-leaf's descendants would need to move too, which
// neither operation attempts.

async fn promote_account(
    State(state): State<AppState>,
    guard: RequirePermission<permcodes::AmendAccount>,
    Path(id): Path<i64>,
) -> Result<StatusCode, (StatusCode, Json<Value>)> {
    let auth = guard.0;
    let mut tx = db::begin(&state.pool, auth.tenant_id)
        .await
        .map_err(|_| internal_error())?;

    let Some(account) = fetch_account(&mut tx, id)
        .await
        .map_err(|_| internal_error())?
    else {
        return Err(not_found());
    };
    if account.child_count > 0 {
        return Err(bad_request("has_children"));
    }
    if account.level == 1 {
        return Err(bad_request("already_top_level"));
    }

    let old_parent = fetch_parent(&mut tx, auth.tenant_id, &account)
        .await
        .map_err(|_| internal_error())?;

    let new_level = account.level - 1;
    let own_code = account.own_code();
    let mut codes = [
        account.general_ledger_code,
        account.subsidiary_code,
        account.analytic1_code,
        account.analytic2_code,
    ];
    codes[(account.level - 1) as usize] = 0; // drop the old deepest segment
    codes[(new_level - 1) as usize] = own_code; // reuse it one level shallower

    let new_parent = if new_level > 1 {
        let probe = AccountRecord {
            level: new_level,
            general_ledger_code: codes[0],
            subsidiary_code: codes[1],
            analytic1_code: codes[2],
            ..account.clone()
        };
        let parent = fetch_parent(&mut tx, auth.tenant_id, &probe)
            .await
            .map_err(|_| internal_error())?;
        if parent.is_none() {
            return Err(internal_error()); // tree corruption — the ancestor must exist
        }
        parent
    } else {
        None
    };

    sqlx::query(
        "UPDATE accounts SET general_ledger_code = $1, subsidiary_code = $2, \
         analytic1_code = $3, analytic2_code = $4, updated_at = now(), updated_by = $5 WHERE id = $6",
    )
    .bind(codes[0])
    .bind(codes[1])
    .bind(codes[2])
    .bind(codes[3])
    .bind(auth.user_id)
    .bind(id)
    .execute(&mut *tx)
    .await
    .map_err(conflict_or_internal)?;

    if let Some(p) = old_parent {
        adjust_child_count(&mut tx, p.id, -1)
            .await
            .map_err(|_| internal_error())?;
    }
    if let Some(p) = new_parent {
        adjust_child_count(&mut tx, p.id, 1)
            .await
            .map_err(|_| internal_error())?;
    }

    audit::record_mutation(
        &mut tx,
        auth.tenant_id,
        "accounts",
        id,
        "update",
        Some(auth.user_id),
        Some(json!({ "level": account.level, "code": own_code })),
        Some(json!({ "level": new_level, "code": own_code })),
    )
    .await
    .map_err(|_| internal_error())?;

    tx.commit().await.map_err(|_| internal_error())?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct DemoteRequest {
    parent_id: i64,
}

async fn demote_account(
    State(state): State<AppState>,
    guard: RequirePermission<permcodes::AmendAccount>,
    Path(id): Path<i64>,
    Json(req): Json<DemoteRequest>,
) -> Result<StatusCode, (StatusCode, Json<Value>)> {
    let auth = guard.0;
    let mut tx = db::begin(&state.pool, auth.tenant_id)
        .await
        .map_err(|_| internal_error())?;

    let Some(account) = fetch_account(&mut tx, id)
        .await
        .map_err(|_| internal_error())?
    else {
        return Err(not_found());
    };
    if account.child_count > 0 {
        return Err(bad_request("has_children"));
    }
    if account.level == 4 {
        return Err(bad_request("already_max_level"));
    }
    let Some(target_parent) = fetch_account(&mut tx, req.parent_id)
        .await
        .map_err(|_| internal_error())?
    else {
        return Err(bad_request("target_parent_not_found"));
    };
    if target_parent.id == account.id {
        return Err(bad_request("invalid_target_level"));
    }
    if target_parent.level != account.level {
        return Err(bad_request("invalid_target_level"));
    }

    let old_parent = fetch_parent(&mut tx, auth.tenant_id, &account)
        .await
        .map_err(|_| internal_error())?;

    let own_code = account.own_code();
    let new_level = account.level + 1;
    let mut codes = [
        target_parent.general_ledger_code,
        target_parent.subsidiary_code,
        target_parent.analytic1_code,
        target_parent.analytic2_code,
    ];
    codes[(new_level - 1) as usize] = own_code;

    sqlx::query(
        "UPDATE accounts SET general_ledger_code = $1, subsidiary_code = $2, \
         analytic1_code = $3, analytic2_code = $4, updated_at = now(), updated_by = $5 WHERE id = $6",
    )
    .bind(codes[0])
    .bind(codes[1])
    .bind(codes[2])
    .bind(codes[3])
    .bind(auth.user_id)
    .bind(id)
    .execute(&mut *tx)
    .await
    .map_err(conflict_or_internal)?;

    if let Some(p) = old_parent {
        adjust_child_count(&mut tx, p.id, -1)
            .await
            .map_err(|_| internal_error())?;
    }
    adjust_child_count(&mut tx, target_parent.id, 1)
        .await
        .map_err(|_| internal_error())?;

    audit::record_mutation(
        &mut tx,
        auth.tenant_id,
        "accounts",
        id,
        "update",
        Some(auth.user_id),
        Some(json!({ "level": account.level, "code": own_code })),
        Some(json!({ "level": new_level, "code": own_code, "parentId": target_parent.id })),
    )
    .await
    .map_err(|_| internal_error())?;

    tx.commit().await.map_err(|_| internal_error())?;
    Ok(StatusCode::NO_CONTENT)
}

// ---- lock / unlock ---------------------------------------------------
//
// "Visible only to administrators" in the legacy (B_Lock.Visible := Dm.Admin,
// 03-02-b.md §2.5) — no Pass_Config permission id, same bucket as 1.3's
// admin-only routes, so `RequireSuperuser` here rather than a future 2.8
// permission (there is no id for 2.8 to wire).

async fn set_locked(
    state: &AppState,
    admin: &AuthUser,
    id: i64,
    locked: bool,
) -> Result<StatusCode, (StatusCode, Json<Value>)> {
    let mut tx = db::begin(&state.pool, admin.tenant_id)
        .await
        .map_err(|_| internal_error())?;

    let Some(account) = fetch_account(&mut tx, id)
        .await
        .map_err(|_| internal_error())?
    else {
        return Err(not_found());
    };

    sqlx::query(
        "UPDATE accounts SET is_locked = $1, updated_at = now(), updated_by = $2 WHERE id = $3",
    )
    .bind(locked)
    .bind(admin.user_id)
    .bind(id)
    .execute(&mut *tx)
    .await
    .map_err(|_| internal_error())?;

    audit::record_mutation(
        &mut tx,
        admin.tenant_id,
        "accounts",
        id,
        "update",
        Some(admin.user_id),
        Some(json!({ "isLocked": account.is_locked })),
        Some(json!({ "isLocked": locked })),
    )
    .await
    .map_err(|_| internal_error())?;

    tx.commit().await.map_err(|_| internal_error())?;
    Ok(StatusCode::NO_CONTENT)
}

async fn lock_account(
    State(state): State<AppState>,
    admin: RequireSuperuser,
    Path(id): Path<i64>,
) -> Result<StatusCode, (StatusCode, Json<Value>)> {
    set_locked(&state, &admin.0, id, true).await
}

async fn unlock_account(
    State(state): State<AppState>,
    admin: RequireSuperuser,
    Path(id): Path<i64>,
) -> Result<StatusCode, (StatusCode, Json<Value>)> {
    set_locked(&state, &admin.0, id, false).await
}

// ---- delete ------------------------------------------------------------

/// 03-02-b.md §2.4's guards, in order: has children (rejected), has postings
/// (stub — see `recode_account`'s doc comment, same gap). The legacy's
/// "deletes the whole subtree if `child_count` is stale" danger doesn't
/// apply here — `child_count` is application-maintained transactionally,
/// never a snapshot recomputed only on delete.
async fn delete_account(
    State(state): State<AppState>,
    guard: RequirePermission<permcodes::DeleteAccount>,
    Path(id): Path<i64>,
) -> Result<StatusCode, (StatusCode, Json<Value>)> {
    let auth = guard.0;
    let mut tx = db::begin(&state.pool, auth.tenant_id)
        .await
        .map_err(|_| internal_error())?;

    let Some(account) = fetch_account(&mut tx, id)
        .await
        .map_err(|_| internal_error())?
    else {
        return Err(not_found());
    };
    if account.child_count > 0 {
        return Err(bad_request("has_children"));
    }

    let parent = fetch_parent(&mut tx, auth.tenant_id, &account)
        .await
        .map_err(|_| internal_error())?;

    sqlx::query("DELETE FROM accounts WHERE id = $1")
        .bind(id)
        .execute(&mut *tx)
        .await
        .map_err(|_| internal_error())?;

    if let Some(p) = parent {
        adjust_child_count(&mut tx, p.id, -1)
            .await
            .map_err(|_| internal_error())?;
    }

    audit::record_mutation(
        &mut tx,
        auth.tenant_id,
        "accounts",
        id,
        "delete",
        Some(auth.user_id),
        Some(json!({ "name": account.name, "code": account.own_code(), "level": account.level })),
        None,
    )
    .await
    .map_err(|_| internal_error())?;

    tx.commit().await.map_err(|_| internal_error())?;
    Ok(StatusCode::NO_CONTENT)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(gl: i32, sub: i32, a1: i32, a2: i32, level: i16, name: &str) -> AccountRecord {
        AccountRecord {
            id: 1,
            general_ledger_code: gl,
            subsidiary_code: sub,
            analytic1_code: a1,
            analytic2_code: a2,
            name: name.to_string(),
            child_count: 0,
            is_locked: false,
            level,
            is_active: level > 1,
            party_id: None,
            address: None,
            phone: None,
            fax: None,
            registration_number: None,
            economic_code: None,
            postal_code: None,
            national_id: None,
        }
    }

    #[test]
    fn renders_all_four_legacy_display_formats() {
        let account = record(1, 11, 1, 1, 4, "Petty Cash");
        let widths = [3, 3, 4, 0];
        let detail = render_display(
            &account,
            &["Assets".into(), "Cash".into(), "On Hand".into()],
            &widths,
        );

        assert_eq!(detail.code_ltr, "1-11-1-1");
        assert_eq!(detail.code_rtl, "1-0001-011-001 Petty Cash");
        assert_eq!(detail.full_name_path, "Assets/Cash/On Hand/Petty Cash");
        assert_eq!(detail.line_name, "1 Petty Cash");
    }

    #[test]
    fn ltr_format_omits_trailing_zero_levels() {
        let account = record(1, 11, 0, 0, 2, "Cash");
        let widths = [3, 3, 4, 0];
        let detail = render_display(&account, &["Assets".into()], &widths);
        assert_eq!(detail.code_ltr, "1-11");
    }
}
