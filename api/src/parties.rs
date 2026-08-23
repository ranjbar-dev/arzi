//! Step 3.1 (docs/phase-3-parties.md §3.1): the party register (legacy
//! Sahamdar) plus `party_account_config` (legacy SahamdarConfig), and the
//! "one link, not two" auto-creation of a party's leaf `accounts` rows —
//! specs/07-parties-and-shareholders/07-02-b.md §2.4, 07-03.md, 07-04-a/b.md
//! and 07-07.md are the behavioural ground truth this module ports.
//!
//! **B18 fix** (11-open-decisions.md): the legacy's `SahamdarConfig.SC_Tik`
//! is a *global* scratch column, mutated by `UPDATE ... SET SC_Tik=0` on
//! every editor open — two concurrent sessions editing different parties
//! stomp each other's tick state (07-07.md §7.4's "serious concurrency
//! defect"). Nothing here persists a tick anywhere: `resolve_ticked` below
//! recomputes "does this party already have a leaf account under this
//! control account" from `accounts` on every request, exactly the fix
//! 07-07.md §7.4 prescribes.
//!
//! **"One link, not two"** (07-04-a.md §4.3): the legacy has a second,
//! never-kept-consistent `Sarfasl.S_Card` FK, written by exactly one manual
//! tool. Not ported — `accounts.party_id` (added nullable by 2.1, FK'd by
//! this step's migration) is the only linkage, and it is only ever set by
//! `ensure_leaf_account` below.
//!
//! **Permission wiring deferred.** `auth::authz::RequirePermission`'s own
//! doc comment names 2.8/4.x/6.7 as the wiring steps; Phase 3 isn't on that
//! list and has no dedicated wiring step of its own in docs/01-roadmap.md.
//! Every route here therefore uses plain `AuthUser` (any authenticated
//! user), same as accounts.rs/vouchers.rs before 2.8 — catalogue ids 1105-
//! 1108 exist (seeded by 1.1) but aren't enforced yet. Lock/unlock are the
//! one exception: 07-04-b.md §4.4 says the lock button is admin-only in the
//! legacy UI, so those two routes use `RequireSuperuser`, matching
//! accounts.rs's identical judgment call for account-lock.

use crate::{
    audit,
    auth::authz::{self, RequireSuperuser},
    auth::AuthUser,
    db, AppState,
};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::{Postgres, Transaction};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(list_parties).post(create_party))
        .route("/{id}", get(get_party).put(update_party))
        .route("/{id}/balance", get(get_party_balance))
        .route("/{id}/card-jari", get(get_card_jari))
        .route("/{id}/lock", post(lock_party))
        .route("/{id}/unlock", post(unlock_party))
        .route("/account-config", get(list_account_config))
        .route(
            "/account-config/seed-defaults",
            post(seed_default_account_config),
        )
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
        Json(json!({ "error": "party_not_found" })),
    )
}
fn bad_request(error: &str) -> (StatusCode, Json<Value>) {
    (StatusCode::BAD_REQUEST, Json(json!({ "error": error })))
}
fn conflict_or_internal(err: sqlx::Error) -> (StatusCode, Json<Value>) {
    if let sqlx::Error::Database(db_err) = &err {
        match db_err.constraint() {
            // V4 (07-03.md #V4/#V3): duplicate card number on create.
            Some("parties_card_number_key") => {
                return (
                    StatusCode::CONFLICT,
                    Json(json!({ "error": "duplicate_card_number" })),
                );
            }
            // V5/V6: duplicate national ID (create) / national ID belonging to another card (update).
            Some("parties_national_id_key") => {
                return (
                    StatusCode::CONFLICT,
                    Json(json!({ "error": "duplicate_national_id" })),
                );
            }
            _ => {}
        }
    }
    internal_error()
}

// ---- shared records ------------------------------------------------------

#[derive(sqlx::FromRow, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct PartyRecord {
    id: i64,
    card_number: i32,
    party_type: String,
    first_name: String,
    last_name: String,
    father_name: Option<String>,
    id_card_number: Option<String>,
    birth_date: Option<String>,
    birth_place: Option<String>,
    id_issue_date: Option<String>,
    id_issue_place: Option<String>,
    national_id: Option<String>,
    postal_code: Option<String>,
    registration_number: Option<String>,
    address: Option<String>,
    mobile: Option<String>,
    tax_status: String,
    is_locked: bool,
}

const PARTY_COLUMNS: &str = "id, card_number, party_type::text, first_name, last_name, \
    father_name, id_card_number, birth_date, birth_place, id_issue_date, id_issue_place, \
    national_id, postal_code, registration_number, address, mobile, tax_status::text, is_locked";

async fn fetch_party(
    tx: &mut Transaction<'_, Postgres>,
    id: i64,
) -> Result<Option<PartyRecord>, sqlx::Error> {
    sqlx::query_as(&format!(
        "SELECT {PARTY_COLUMNS} FROM parties WHERE id = $1"
    ))
    .bind(id)
    .fetch_optional(&mut **tx)
    .await
}

/// The generated-leaf-account name formula (07-02-b.md §2.4's table):
/// `"first last-father"` for a person, `"first last"` for a legal entity.
fn generated_account_name(
    party_type: &str,
    first: &str,
    last: &str,
    father: Option<&str>,
) -> String {
    if party_type == "legal_entity" {
        format!("{first} {last}")
    } else {
        format!("{first} {last}-{}", father.unwrap_or(""))
    }
}

/// `pub(crate)` — reused by party_balance.rs (`Jari_Rem` needs the same
/// coordinate resolution as leaf-account creation does).
#[derive(sqlx::FromRow, Clone)]
pub(crate) struct ConfigRow {
    pub(crate) id: i64,
    pub(crate) control_kol_code: i32,
    pub(crate) control_moein_code: i32,
    pub(crate) fixed_tafsil1_code: i32,
    pub(crate) name: String,
    for_person: bool,
    for_legal_entity: bool,
    offered_by_default: bool,
    pub(crate) counts_toward_balance: bool,
}

const CONFIG_COLUMNS: &str = "id, control_kol_code, control_moein_code, fixed_tafsil1_code, name, \
    for_person, for_legal_entity, offered_by_default, counts_toward_balance";

pub(crate) async fn fetch_all_config(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: i64,
) -> Result<Vec<ConfigRow>, sqlx::Error> {
    sqlx::query_as(&format!(
        "SELECT {CONFIG_COLUMNS} FROM party_account_config WHERE tenant_id = $1 ORDER BY id"
    ))
    .bind(tenant_id)
    .fetch_all(&mut **tx)
    .await
}

/// The coordinate a party's leaf account occupies under this control
/// account: Tafsil-1 = the card number when `fixed_tafsil1_code = 0`,
/// otherwise Tafsil-2 under the fixed Tafsil-1 (07-07.md §7.2's `SC_T` rule,
/// the case 07-02-b.md §2.4 flags the legacy as never actually reaching).
pub(crate) fn coordinate(config: &ConfigRow, card_number: i32) -> (i32, i32, i32, i32) {
    if config.fixed_tafsil1_code == 0 {
        (
            config.control_kol_code,
            config.control_moein_code,
            card_number,
            0,
        )
    } else {
        (
            config.control_kol_code,
            config.control_moein_code,
            config.fixed_tafsil1_code,
            card_number,
        )
    }
}

pub(crate) async fn find_account_id_by_codes(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: i64,
    codes: (i32, i32, i32, i32),
) -> Result<Option<i64>, sqlx::Error> {
    sqlx::query_scalar(
        "SELECT id FROM accounts WHERE tenant_id = $1 AND general_ledger_code = $2 \
         AND subsidiary_code = $3 AND analytic1_code = $4 AND analytic2_code = $5",
    )
    .bind(tenant_id)
    .bind(codes.0)
    .bind(codes.1)
    .bind(codes.2)
    .bind(codes.3)
    .fetch_optional(&mut **tx)
    .await
}

/// "Does this party already have a detail account here" — computed live,
/// never stored (the B18 fix). Returns `config.id -> Some(account_id)` for
/// every config row that already resolves to a real leaf.
async fn resolve_ticked(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: i64,
    configs: &[ConfigRow],
    card_number: i32,
) -> Result<std::collections::HashMap<i64, i64>, sqlx::Error> {
    let mut found = std::collections::HashMap::new();
    for config in configs {
        if let Some(account_id) =
            find_account_id_by_codes(tx, tenant_id, coordinate(config, card_number)).await?
        {
            found.insert(config.id, account_id);
        }
    }
    Ok(found)
}

enum EnsureLeafError {
    ControlAccountNotProvisioned { config_name: String },
    Db,
}
impl From<sqlx::Error> for EnsureLeafError {
    fn from(_: sqlx::Error) -> Self {
        EnsureLeafError::Db
    }
}

/// Creates (or, if it already exists, no-ops and returns) the leaf `accounts`
/// row for one ticked control account. The control account itself (the Moein
/// node, or the fixed Tafsil-1 node in the `fixed_tafsil1_code > 0` case)
/// must already exist in the tenant's chart of accounts — this function
/// attaches a detail account under it, it does not provision the control
/// structure (same "no tenant-provisioning flow" boundary 2.1 documented for
/// `account_code_format`).
async fn ensure_leaf_account(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: i64,
    party_id: i64,
    config: &ConfigRow,
    card_number: i32,
    account_name: &str,
    actor_id: i64,
) -> Result<i64, EnsureLeafError> {
    let codes = coordinate(config, card_number);
    if let Some(existing_id) = find_account_id_by_codes(tx, tenant_id, codes).await? {
        return Ok(existing_id);
    }

    let parent_codes = if config.fixed_tafsil1_code == 0 {
        (config.control_kol_code, config.control_moein_code, 0, 0)
    } else {
        (
            config.control_kol_code,
            config.control_moein_code,
            config.fixed_tafsil1_code,
            0,
        )
    };
    let Some(parent_id) = find_account_id_by_codes(tx, tenant_id, parent_codes).await? else {
        return Err(EnsureLeafError::ControlAccountNotProvisioned {
            config_name: config.name.clone(),
        });
    };

    let account_id: i64 = sqlx::query_scalar(
        "INSERT INTO accounts \
         (tenant_id, general_ledger_code, subsidiary_code, analytic1_code, analytic2_code, name, \
          party_id, created_by) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8) RETURNING id",
    )
    .bind(tenant_id)
    .bind(codes.0)
    .bind(codes.1)
    .bind(codes.2)
    .bind(codes.3)
    .bind(account_name)
    .bind(party_id)
    .bind(actor_id)
    .fetch_one(&mut **tx)
    .await?;

    sqlx::query(
        "UPDATE accounts SET child_count = child_count + 1, updated_at = now() WHERE id = $1",
    )
    .bind(parent_id)
    .execute(&mut **tx)
    .await?;

    audit::record_mutation(
        tx,
        tenant_id,
        "accounts",
        account_id,
        "insert",
        Some(actor_id),
        None,
        Some(json!({ "codes": codes, "name": account_name, "partyId": party_id })),
    )
    .await?;

    Ok(account_id)
}

/// Unticking a control account (docs/phase-3-parties.md §3.1's Build bullet):
/// delete the leaf if it has no postings, otherwise leave it in place (it's
/// no longer offered as ticked next time, but real transaction history isn't
/// destroyed) — a genuinely new rule, the legacy's save loop can never delete
/// at all (07-07.md §7.5's "§12-Q17").
async fn maybe_delete_leaf_account(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: i64,
    account_id: i64,
    actor_id: i64,
) -> Result<(), sqlx::Error> {
    let has_postings: bool =
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM voucher_lines WHERE account_id = $1)")
            .bind(account_id)
            .fetch_one(&mut **tx)
            .await?;
    if has_postings {
        return Ok(());
    }

    let row: Option<(i32, i32, i32, i32, String)> = sqlx::query_as(
        "SELECT general_ledger_code, subsidiary_code, analytic1_code, analytic2_code, name \
         FROM accounts WHERE id = $1",
    )
    .bind(account_id)
    .fetch_optional(&mut **tx)
    .await?;
    let Some((gl, sub, a1, a2, name)) = row else {
        return Ok(()); // already gone
    };

    let parent_codes = if a2 > 0 {
        (gl, sub, a1, 0)
    } else {
        (gl, sub, 0, 0)
    };
    let parent_id = find_account_id_by_codes(tx, tenant_id, parent_codes).await?;

    sqlx::query("DELETE FROM accounts WHERE id = $1")
        .bind(account_id)
        .execute(&mut **tx)
        .await?;
    if let Some(pid) = parent_id {
        sqlx::query(
            "UPDATE accounts SET child_count = child_count - 1, updated_at = now() WHERE id = $1",
        )
        .bind(pid)
        .execute(&mut **tx)
        .await?;
    }

    audit::record_mutation(
        tx,
        tenant_id,
        "accounts",
        account_id,
        "delete",
        Some(actor_id),
        Some(json!({ "codes": [gl, sub, a1, a2], "name": name })),
        None,
    )
    .await?;
    Ok(())
}

// ---- validation ------------------------------------------------------------

/// V1-V3 (07-03.md §3.2/§3.3): first/last name non-empty for both kinds,
/// father's name additionally required for a natural person.
fn validate_names(
    party_type: &str,
    first: &str,
    last: &str,
    father: Option<&str>,
) -> Result<(), &'static str> {
    if first.trim().is_empty() || last.trim().is_empty() {
        return Err("incomplete_data");
    }
    if party_type != "legal_entity" && father.map(str::trim).unwrap_or("").is_empty() {
        return Err("incomplete_data");
    }
    Ok(())
}

/// Every requested control-account id must be a real config row for this
/// tenant, offered for this party's kind (`for_person`/`for_legal_entity`).
fn validate_config_ids<'a>(
    requested: &[i64],
    configs: &'a [ConfigRow],
    party_type: &str,
) -> Result<Vec<&'a ConfigRow>, &'static str> {
    requested
        .iter()
        .map(|id| {
            configs
                .iter()
                .find(|c| c.id == *id)
                .filter(|c| {
                    if party_type == "legal_entity" {
                        c.for_legal_entity
                    } else {
                        c.for_person
                    }
                })
                .ok_or("invalid_control_account")
        })
        .collect()
}

// ---- create ------------------------------------------------------------

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PartyFields {
    card_number: i32,
    party_type: String, // "natural_person" | "legal_entity"
    first_name: String,
    last_name: String,
    father_name: Option<String>,
    id_card_number: Option<String>,
    birth_date: Option<String>,
    birth_place: Option<String>,
    id_issue_date: Option<String>,
    id_issue_place: Option<String>,
    national_id: Option<String>,
    postal_code: Option<String>,
    registration_number: Option<String>,
    address: Option<String>,
    mobile: Option<String>,
    tax_status: Option<String>,
    control_account_config_ids: Vec<i64>,
}

async fn create_party(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<PartyFields>,
) -> Result<(StatusCode, Json<Value>), (StatusCode, Json<Value>)> {
    if req.party_type != "natural_person" && req.party_type != "legal_entity" {
        return Err(bad_request("invalid_party_type"));
    }
    validate_names(
        &req.party_type,
        &req.first_name,
        &req.last_name,
        req.father_name.as_deref(),
    )
    .map_err(bad_request)?;
    let tax_status = req
        .tax_status
        .clone()
        .unwrap_or_else(|| "not_specified".to_string());

    let mut tx = db::begin(&state.pool, auth.tenant_id)
        .await
        .map_err(|_| internal_error())?;

    let configs = fetch_all_config(&mut tx, auth.tenant_id)
        .await
        .map_err(|_| internal_error())?;
    let ticked = validate_config_ids(&req.control_account_config_ids, &configs, &req.party_type)
        .map_err(bad_request)?;

    let id: i64 = sqlx::query_scalar(
        "INSERT INTO parties \
         (tenant_id, card_number, party_type, first_name, last_name, father_name, id_card_number, \
          birth_date, birth_place, id_issue_date, id_issue_place, national_id, postal_code, \
          registration_number, address, mobile, tax_status, created_by) \
         VALUES ($1, $2, $3::party_type, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, \
                 $17::party_tax_status, $18) RETURNING id",
    )
    .bind(auth.tenant_id)
    .bind(req.card_number)
    .bind(&req.party_type)
    .bind(req.first_name.trim())
    .bind(req.last_name.trim())
    .bind(req.father_name.as_deref().map(str::trim))
    .bind(&req.id_card_number)
    .bind(&req.birth_date)
    .bind(&req.birth_place)
    .bind(&req.id_issue_date)
    .bind(&req.id_issue_place)
    .bind(&req.national_id)
    .bind(&req.postal_code)
    .bind(&req.registration_number)
    .bind(&req.address)
    .bind(&req.mobile)
    .bind(&tax_status)
    .bind(auth.user_id)
    .fetch_one(&mut *tx)
    .await
    .map_err(conflict_or_internal)?;

    let account_name = generated_account_name(
        &req.party_type,
        req.first_name.trim(),
        req.last_name.trim(),
        req.father_name.as_deref().map(str::trim),
    );
    for config in ticked {
        ensure_leaf_account(&mut tx, auth.tenant_id, id, config, req.card_number, &account_name, auth.user_id)
            .await
            .map_err(|e| match e {
                EnsureLeafError::ControlAccountNotProvisioned { config_name } => {
                    (StatusCode::BAD_REQUEST, Json(json!({ "error": "control_account_not_provisioned", "controlAccount": config_name })))
                }
                EnsureLeafError::Db => internal_error(),
            })?;
    }

    audit::record_mutation(
        &mut tx,
        auth.tenant_id,
        "parties",
        id,
        "insert",
        Some(auth.user_id),
        None,
        Some(json!({ "cardNumber": req.card_number, "partyType": req.party_type })),
    )
    .await
    .map_err(|_| internal_error())?;

    tx.commit().await.map_err(|_| internal_error())?;
    Ok((StatusCode::CREATED, Json(json!({ "id": id }))))
}

// ---- update --------------------------------------------------------------

async fn update_party(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
    Json(req): Json<PartyFields>,
) -> Result<StatusCode, (StatusCode, Json<Value>)> {
    let mut tx = db::begin(&state.pool, auth.tenant_id)
        .await
        .map_err(|_| internal_error())?;

    let Some(existing) = fetch_party(&mut tx, id)
        .await
        .map_err(|_| internal_error())?
    else {
        return Err(not_found());
    };
    // party_type is immutable here — the legacy's person<->company move
    // (07-04-b.md §4.5) is a separate hidden admin tool with no manual test
    // or Build bullet in this step's scope; not ported.
    validate_names(
        &existing.party_type,
        &req.first_name,
        &req.last_name,
        req.father_name.as_deref(),
    )
    .map_err(bad_request)?;
    let tax_status = req
        .tax_status
        .clone()
        .unwrap_or_else(|| "not_specified".to_string());

    let configs = fetch_all_config(&mut tx, auth.tenant_id)
        .await
        .map_err(|_| internal_error())?;
    let desired = validate_config_ids(
        &req.control_account_config_ids,
        &configs,
        &existing.party_type,
    )
    .map_err(bad_request)?;
    let desired_ids: std::collections::HashSet<i64> = desired.iter().map(|c| c.id).collect();

    let currently_ticked = resolve_ticked(&mut tx, auth.tenant_id, &configs, existing.card_number)
        .await
        .map_err(|_| internal_error())?;

    sqlx::query(
        "UPDATE parties SET first_name = $1, last_name = $2, father_name = $3, id_card_number = $4, \
         birth_date = $5, birth_place = $6, id_issue_date = $7, id_issue_place = $8, national_id = $9, \
         postal_code = $10, registration_number = $11, address = $12, mobile = $13, \
         tax_status = $14::party_tax_status, updated_at = now(), updated_by = $15 WHERE id = $16",
    )
    .bind(req.first_name.trim())
    .bind(req.last_name.trim())
    .bind(req.father_name.as_deref().map(str::trim))
    .bind(&req.id_card_number)
    .bind(&req.birth_date)
    .bind(&req.birth_place)
    .bind(&req.id_issue_date)
    .bind(&req.id_issue_place)
    .bind(&req.national_id)
    .bind(&req.postal_code)
    .bind(&req.registration_number)
    .bind(&req.address)
    .bind(&req.mobile)
    .bind(&tax_status)
    .bind(auth.user_id)
    .bind(id)
    .execute(&mut *tx)
    .await
    .map_err(conflict_or_internal)?;

    let account_name = generated_account_name(
        &existing.party_type,
        req.first_name.trim(),
        req.last_name.trim(),
        req.father_name.as_deref().map(str::trim),
    );

    for config in &desired {
        if !currently_ticked.contains_key(&config.id) {
            ensure_leaf_account(
                &mut tx,
                auth.tenant_id,
                id,
                config,
                existing.card_number,
                &account_name,
                auth.user_id,
            )
            .await
            .map_err(|e| match e {
                EnsureLeafError::ControlAccountNotProvisioned { config_name } => (
                    StatusCode::BAD_REQUEST,
                    Json(json!({ "error": "control_account_not_provisioned", "controlAccount": config_name })),
                ),
                EnsureLeafError::Db => internal_error(),
            })?;
        }
    }
    for (config_id, account_id) in &currently_ticked {
        if !desired_ids.contains(config_id) {
            maybe_delete_leaf_account(&mut tx, auth.tenant_id, *account_id, auth.user_id)
                .await
                .map_err(|_| internal_error())?;
        }
    }

    audit::record_mutation(
        &mut tx,
        auth.tenant_id,
        "parties",
        id,
        "update",
        Some(auth.user_id),
        Some(json!({ "firstName": existing.first_name, "lastName": existing.last_name })),
        Some(json!({ "firstName": req.first_name.trim(), "lastName": req.last_name.trim() })),
    )
    .await
    .map_err(|_| internal_error())?;

    tx.commit().await.map_err(|_| internal_error())?;
    Ok(StatusCode::NO_CONTENT)
}

// ---- list / get ------------------------------------------------------------

#[derive(Deserialize)]
struct ListQuery {
    kind: Option<String>,
}

async fn list_parties(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(params): Query<ListQuery>,
) -> Result<Json<Vec<PartyRecord>>, (StatusCode, Json<Value>)> {
    let mut tx = db::begin(&state.pool, auth.tenant_id)
        .await
        .map_err(|_| internal_error())?;
    let rows: Vec<PartyRecord> = match params.kind.as_deref() {
        Some(kind) if kind == "natural_person" || kind == "legal_entity" => sqlx::query_as(&format!(
            "SELECT {PARTY_COLUMNS} FROM parties WHERE tenant_id = $1 AND party_type = $2::party_type \
             ORDER BY last_name, first_name"
        ))
        .bind(auth.tenant_id)
        .bind(kind)
        .fetch_all(&mut *tx)
        .await
        .map_err(|_| internal_error())?,
        _ => sqlx::query_as(&format!(
            "SELECT {PARTY_COLUMNS} FROM parties WHERE tenant_id = $1 ORDER BY last_name, first_name"
        ))
        .bind(auth.tenant_id)
        .fetch_all(&mut *tx)
        .await
        .map_err(|_| internal_error())?,
    };
    tx.rollback().await.ok();
    Ok(Json(rows))
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ControlAccountView {
    config_id: i64,
    name: String,
    control_kol_code: i32,
    control_moein_code: i32,
    fixed_tafsil1_code: i32,
    counts_toward_balance: bool,
    ticked: bool,
    account_id: Option<i64>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PartyDetail {
    #[serde(flatten)]
    party: PartyRecord,
    control_accounts: Vec<ControlAccountView>,
}

async fn get_party(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<Json<PartyDetail>, (StatusCode, Json<Value>)> {
    let mut tx = db::begin(&state.pool, auth.tenant_id)
        .await
        .map_err(|_| internal_error())?;
    let Some(party) = fetch_party(&mut tx, id)
        .await
        .map_err(|_| internal_error())?
    else {
        return Err(not_found());
    };

    let configs = fetch_all_config(&mut tx, auth.tenant_id)
        .await
        .map_err(|_| internal_error())?;
    let ticked = resolve_ticked(&mut tx, auth.tenant_id, &configs, party.card_number)
        .await
        .map_err(|_| internal_error())?;
    tx.rollback().await.ok();

    // 07-07.md §7.4's selection rule: offer the default set for this party's
    // kind, plus every control account where the party already has an
    // account (never silently drop an existing non-default one).
    let offered_flag = |c: &ConfigRow| {
        if party.party_type == "legal_entity" {
            c.for_legal_entity
        } else {
            c.for_person
        }
    };
    let control_accounts = configs
        .into_iter()
        .filter(|c| (offered_flag(c) && c.offered_by_default) || ticked.contains_key(&c.id))
        .map(|c| ControlAccountView {
            config_id: c.id,
            name: c.name.clone(),
            control_kol_code: c.control_kol_code,
            control_moein_code: c.control_moein_code,
            fixed_tafsil1_code: c.fixed_tafsil1_code,
            counts_toward_balance: c.counts_toward_balance,
            ticked: ticked.contains_key(&c.id),
            account_id: ticked.get(&c.id).copied(),
        })
        .collect();

    Ok(Json(PartyDetail {
        party,
        control_accounts,
    }))
}

// ---- balance (Jari) -------------------------------------------------------

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct BalanceQuery {
    fiscal_year_id: i64,
}

/// docs/phase-3-parties.md §3.2's Build bullet: the balance is computed for
/// a **chosen** fiscal year, an explicit required parameter, never the
/// session's implicit current year (07-06-a.md §6.2's note that the legacy
/// form lets a user inspect any year).
async fn get_party_balance(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
    Query(params): Query<BalanceQuery>,
) -> Result<Json<crate::party_balance::PartyBalance>, (StatusCode, Json<Value>)> {
    let mut tx = db::begin(&state.pool, auth.tenant_id)
        .await
        .map_err(|_| internal_error())?;
    let Some(party) = fetch_party(&mut tx, id)
        .await
        .map_err(|_| internal_error())?
    else {
        return Err(not_found());
    };

    let balance = crate::party_balance::compute_balance(
        &mut tx,
        auth.tenant_id,
        party.card_number,
        params.fiscal_year_id,
    )
    .await
    .map_err(|_| internal_error())?;
    tx.rollback().await.ok();

    Ok(Json(balance))
}

// ---- Card Jari (step 6.3) --------------------------------------------------

/// Step 6.3 (docs/phase-6-reporting.md §6.3): "reusing 3.2's `Jari_Rem`-
/// equivalent function directly -- no separate reimplementation." This
/// handler delegates entirely to `party_balance::compute_balance` (the SAME
/// function `get_party_balance` above calls) and adds nothing to the balance
/// math itself -- its only job is composing each breakdown row with a ready-
/// made link into 6.2's ledger endpoint, "drill-through into the subsidiary
/// ledger for any control account in the breakdown," so a caller never has
/// to reconstruct that query itself. `ledgerHref` scopes to the SAME
/// fiscal year's own `[start_date, end_date]` (04-04-a.md §4.1: Card Jari
/// "has no date range at all... always inception-to-date for that year").
async fn get_card_jari(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
    Query(params): Query<BalanceQuery>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    authz::require(&auth, "card_summary")?;
    let mut tx = db::begin(&state.pool, auth.tenant_id)
        .await
        .map_err(|_| internal_error())?;
    let Some(party) = fetch_party(&mut tx, id)
        .await
        .map_err(|_| internal_error())?
    else {
        return Err(not_found());
    };
    let Some((start_date, end_date)): Option<(chrono::NaiveDate, chrono::NaiveDate)> =
        sqlx::query_as(
            "SELECT start_date, end_date FROM fiscal_years WHERE tenant_id = $1 AND id = $2",
        )
        .bind(auth.tenant_id)
        .bind(params.fiscal_year_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|_| internal_error())?
    else {
        tx.rollback().await.ok();
        return Err((
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "fiscal_year_not_found" })),
        ));
    };

    let balance = crate::party_balance::compute_balance(
        &mut tx,
        auth.tenant_id,
        party.card_number,
        params.fiscal_year_id,
    )
    .await
    .map_err(|_| internal_error())?;
    tx.rollback().await.ok();

    let breakdown: Vec<Value> = balance
        .breakdown
        .iter()
        .map(|row| {
            json!({
                "configId": row.config_id,
                "name": row.name,
                "accountId": row.account_id,
                "debit": row.debit,
                "credit": row.credit,
                "remainder": row.remainder,
                "ledgerHref": format!(
                    "/api/v1/reports/ledger?accountIds={}&fiscalYearId={}&fromDate={start_date}&toDate={end_date}",
                    row.account_id, params.fiscal_year_id
                ),
            })
        })
        .collect();

    Ok(Json(json!({
        "partyId": party.id,
        "cardNumber": party.card_number,
        "fiscalYearId": params.fiscal_year_id,
        "total": balance.total,
        "breakdown": breakdown,
    })))
}

// ---- lock / unlock ---------------------------------------------------
//
// Admin-only in the legacy (07-04-b.md §4.4: "The lock button itself is
// admin-only", SahamdarU.pas:112-113/:127-128) — same judgment call as
// accounts.rs's account-lock: `RequireSuperuser`, no catalogue id.

async fn set_locked(
    state: &AppState,
    admin: &AuthUser,
    id: i64,
    locked: bool,
) -> Result<StatusCode, (StatusCode, Json<Value>)> {
    let mut tx = db::begin(&state.pool, admin.tenant_id)
        .await
        .map_err(|_| internal_error())?;
    let Some(party) = fetch_party(&mut tx, id)
        .await
        .map_err(|_| internal_error())?
    else {
        return Err(not_found());
    };

    sqlx::query(
        "UPDATE parties SET is_locked = $1, updated_at = now(), updated_by = $2 WHERE id = $3",
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
        "parties",
        id,
        "update",
        Some(admin.user_id),
        Some(json!({ "isLocked": party.is_locked })),
        Some(json!({ "isLocked": locked })),
    )
    .await
    .map_err(|_| internal_error())?;

    tx.commit().await.map_err(|_| internal_error())?;
    Ok(StatusCode::NO_CONTENT)
}

async fn lock_party(
    State(state): State<AppState>,
    admin: RequireSuperuser,
    Path(id): Path<i64>,
) -> Result<StatusCode, (StatusCode, Json<Value>)> {
    set_locked(&state, &admin.0, id, true).await
}

async fn unlock_party(
    State(state): State<AppState>,
    admin: RequireSuperuser,
    Path(id): Path<i64>,
) -> Result<StatusCode, (StatusCode, Json<Value>)> {
    set_locked(&state, &admin.0, id, false).await
}

// ---- party_account_config -------------------------------------------------

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ConfigView {
    id: i64,
    control_kol_code: i32,
    control_moein_code: i32,
    fixed_tafsil1_code: i32,
    name: String,
    for_person: bool,
    for_legal_entity: bool,
    offered_by_default: bool,
    counts_toward_balance: bool,
}
impl From<ConfigRow> for ConfigView {
    fn from(c: ConfigRow) -> Self {
        ConfigView {
            id: c.id,
            control_kol_code: c.control_kol_code,
            control_moein_code: c.control_moein_code,
            fixed_tafsil1_code: c.fixed_tafsil1_code,
            name: c.name,
            for_person: c.for_person,
            for_legal_entity: c.for_legal_entity,
            offered_by_default: c.offered_by_default,
            counts_toward_balance: c.counts_toward_balance,
        }
    }
}

async fn list_account_config(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Vec<ConfigView>>, (StatusCode, Json<Value>)> {
    let mut tx = db::begin(&state.pool, auth.tenant_id)
        .await
        .map_err(|_| internal_error())?;
    let rows = fetch_all_config(&mut tx, auth.tenant_id)
        .await
        .map_err(|_| internal_error())?;
    tx.rollback().await.ok();
    Ok(Json(rows.into_iter().map(ConfigView::from).collect()))
}

/// The 10 standard control-account rows from 07-07.md §7.3 (11 legacy rows,
/// minus the 104-002 collision merged into one — see the comment on the
/// array below), decoded from the legacy's `Coding` design-time snapshots.
/// Idempotent (`ON CONFLICT DO NOTHING` on the natural key) — safe to call
/// more than once. There is no
/// tenant-provisioning flow yet to call this automatically (same gap 2.1
/// documented for `account_code_format`), so it's a superuser-triggered
/// admin action for now.
const DEFAULT_CONFIG_SEED: &[(i32, i32, &str, bool, bool)] = &[
    // (kol, moein, name, for_person, for_legal_entity)
    (103, 1, "Trade accounts receivable — persons", true, false),
    (104, 1, "Accounts receivable — personnel", true, false),
    // 104-002 is seeded ONCE, offered to both kinds — 07-07.md §7.3 flags this
    // Kol/Moein pair as colliding across the person set ("tenants") and the
    // legal-entity set ("companies") in the legacy's own design-time snapshot
    // ("the seed blobs are stale relative to whatever the production table
    // now holds"). One control account can't occupy the same coordinate
    // twice under this table's natural key, so the two are merged here
    // rather than silently reproducing the stale duplicate.
    (
        104,
        2,
        "Accounts receivable — tenants / companies",
        true,
        true,
    ),
    (109, 1, "Guarantee notes receivable — persons", true, false),
    (301, 1, "Trade accounts payable — persons", true, false),
    (301, 3, "Trade notes payable — persons", true, false),
    (103, 2, "Trade accounts receivable — companies", false, true),
    (
        109,
        2,
        "Guarantee notes receivable — companies",
        false,
        true,
    ),
    (301, 2, "Trade accounts payable — companies", false, true),
    (301, 4, "Trade notes payable — companies", false, true),
];

async fn seed_default_account_config(
    State(state): State<AppState>,
    admin: RequireSuperuser,
) -> Result<StatusCode, (StatusCode, Json<Value>)> {
    let auth = admin.0;
    let mut tx = db::begin(&state.pool, auth.tenant_id)
        .await
        .map_err(|_| internal_error())?;

    for (kol, moein, name, for_person, for_legal_entity) in DEFAULT_CONFIG_SEED {
        sqlx::query(
            "INSERT INTO party_account_config \
             (tenant_id, control_kol_code, control_moein_code, fixed_tafsil1_code, name, for_person, \
              for_legal_entity, offered_by_default, counts_toward_balance) \
             VALUES ($1, $2, $3, 0, $4, $5, $6, true, true) \
             ON CONFLICT (tenant_id, control_kol_code, control_moein_code, fixed_tafsil1_code) DO NOTHING",
        )
        .bind(auth.tenant_id)
        .bind(kol)
        .bind(moein)
        .bind(name)
        .bind(for_person)
        .bind(for_legal_entity)
        .execute(&mut *tx)
        .await
        .map_err(|_| internal_error())?;
    }

    tx.commit().await.map_err(|_| internal_error())?;
    Ok(StatusCode::NO_CONTENT)
}
