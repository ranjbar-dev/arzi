//! Step 5.1 (docs/phase-5-inventory.md §5.1): one merged item master, warehouse model and unit-
//! of-measure table — replacing the legacy's two structurally incompatible inventory subsystems
//! (specs/05-inventory/05-01-entity-model.md §1.0's "do not build two and reconcile later").
//! `05-02-a/b-item-master-crud-rules.md` §2.7's port-as-is summary is the behavioural ground
//! truth for `items`; `warehouses`/`units_of_measure`/`pistachio_grades` have no legacy CRUD
//! screen at all (§1.1/§1.3/§1.4) so their validation rules are this step's own, kept minimal.
//!
//! **Judgment call, permission wiring deferred** — same as every Phase 3/4 module: no dedicated
//! wiring step exists for Phase 5 in the roadmap (unlike 2.8's accounting-core wiring), so every
//! route below uses plain `AuthUser` except a superuser-only seed action. Catalogue ids 1401
//! (`warehouse settings`) / 1402 (`item master`) exist (seeded by 1.1) but aren't enforced yet —
//! a real gap, flagged not hidden, same as 3.1/4.1's own notes on this.
//!
//! **Judgment call, UI scope** — this step's manual test talks about being "usable from the UI",
//! but every other schema+API step in this project (2.1, 3.1, 4.1) ships its CRUD as a tested API
//! surface only and defers the actual screens to a dedicated later step (2.2/3.4/4.5); Phase 5 has
//! its own dedicated screens step, 5.9. Read literally, "usable from the UI" contrasts with the
//! legacy's "must populate by direct SQL only" (§1.3) — an application-level API satisfies that
//! contrast even before 5.9's screens exist. Followed the established pattern: API + tests here,
//! full interactive UI in 5.9.
//!
//! **Judgment call, low-stock alert** — the Build bullet's 4th bullet ("surface it as a real
//! low-stock indicator") explicitly depends on 5.3's stock-on-hand query, which doesn't exist
//! yet; `min_stock` is stored here (replacing the legacy's display-only `AJ_Alarm`) but the alert
//! itself is out of this step's reach, exactly as the roadmap allows ("once 5.3's stock query
//! exists").

use crate::{audit, auth::authz::RequireSuperuser, auth::AuthUser, db, AppState};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{get, post, put},
    Json, Router,
};
use bigdecimal::BigDecimal;
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::{Postgres, Transaction};

fn internal_error() -> (StatusCode, Json<Value>) {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(json!({ "error": "internal_error" })),
    )
}
fn bad_request(error: &str) -> (StatusCode, Json<Value>) {
    (StatusCode::BAD_REQUEST, Json(json!({ "error": error })))
}
fn not_found(error: &str) -> (StatusCode, Json<Value>) {
    (StatusCode::NOT_FOUND, Json(json!({ "error": error })))
}
fn conflict(error: &str) -> (StatusCode, Json<Value>) {
    (StatusCode::CONFLICT, Json(json!({ "error": error })))
}
fn map_unique_violation(
    err: sqlx::Error,
    constraint: &str,
    error: &str,
) -> (StatusCode, Json<Value>) {
    if let sqlx::Error::Database(db_err) = &err {
        if db_err.constraint() == Some(constraint) {
            return conflict(error);
        }
    }
    internal_error()
}

/// Every warehouse posting-account link must resolve to a real leaf account. Not itself a legacy
/// rule (§1.1 lists no such check) — but 5.8's Phase 2.5 engine enforces leaf-only postings
/// unconditionally regardless of caller, same reasoning 4.4 already documented for deposit slips'
/// payer/bank accounts: the pre-check just surfaces the engine's own inevitable future rejection
/// as a friendly domain error now, instead of a generic one once postings exist.
async fn require_leaf_account(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: i64,
    account_id: i64,
    field: &str,
) -> Result<(), (StatusCode, Json<Value>)> {
    let child_count: Option<i32> =
        sqlx::query_scalar("SELECT child_count FROM accounts WHERE tenant_id = $1 AND id = $2")
            .bind(tenant_id)
            .bind(account_id)
            .fetch_optional(&mut **tx)
            .await
            .map_err(|_| internal_error())?;
    match child_count {
        None => Err(bad_request(&format!("{field}_not_found"))),
        Some(c) if c > 0 => Err(bad_request(&format!("{field}_not_leaf"))),
        _ => Ok(()),
    }
}

// =========================================================================
// warehouses   <- legacy Anbar_Config (§1.1)
// =========================================================================

pub fn warehouses_router() -> Router<AppState> {
    Router::new()
        .route("/", get(list_warehouses).post(create_warehouse))
        .route("/{id}", get(get_warehouse).put(update_warehouse))
        .route("/{id}/activate", post(activate_warehouse))
        .route("/{id}/deactivate", post(deactivate_warehouse))
}

#[derive(sqlx::FromRow, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct WarehouseRecord {
    id: i64,
    name: String,
    vat_rate_pct: BigDecimal,
    purchase_account_id: i64,
    purchase_return_account_id: i64,
    sales_account_id: i64,
    sales_return_account_id: i64,
    discount_account_id: i64,
    vat_account_id: i64,
    is_active: bool,
    /// 5.8: production's two roles and transfer's wash-entry role — nullable, since not every
    /// warehouse does production or is a transfer endpoint (posting fails with a clear error if
    /// used without one, `inventory_posting.rs`).
    finished_goods_account_id: Option<i64>,
    raw_materials_account_id: Option<i64>,
    inventory_account_id: Option<i64>,
}

const WAREHOUSE_COLUMNS: &str = "id, name, vat_rate_pct, purchase_account_id, \
    purchase_return_account_id, sales_account_id, sales_return_account_id, discount_account_id, \
    vat_account_id, is_active, finished_goods_account_id, raw_materials_account_id, inventory_account_id";

#[derive(Deserialize)]
struct ListWarehousesQuery {
    #[serde(default)]
    active_only: bool,
}

async fn list_warehouses(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(q): Query<ListWarehousesQuery>,
) -> Result<Json<Vec<WarehouseRecord>>, (StatusCode, Json<Value>)> {
    let mut tx = db::begin(&state.pool, auth.tenant_id)
        .await
        .map_err(|_| internal_error())?;
    let sql = format!(
        "SELECT {WAREHOUSE_COLUMNS} FROM warehouses WHERE tenant_id = $1 {} ORDER BY name",
        if q.active_only { "AND is_active" } else { "" }
    );
    let rows = sqlx::query_as(&sql)
        .bind(auth.tenant_id)
        .fetch_all(&mut *tx)
        .await
        .map_err(|_| internal_error())?;
    tx.rollback().await.ok();
    Ok(Json(rows))
}

async fn fetch_warehouse(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: i64,
    id: i64,
) -> Result<Option<WarehouseRecord>, sqlx::Error> {
    sqlx::query_as(&format!(
        "SELECT {WAREHOUSE_COLUMNS} FROM warehouses WHERE tenant_id = $1 AND id = $2"
    ))
    .bind(tenant_id)
    .bind(id)
    .fetch_optional(&mut **tx)
    .await
}

async fn get_warehouse(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<Json<WarehouseRecord>, (StatusCode, Json<Value>)> {
    let mut tx = db::begin(&state.pool, auth.tenant_id)
        .await
        .map_err(|_| internal_error())?;
    let Some(row) = fetch_warehouse(&mut tx, auth.tenant_id, id)
        .await
        .map_err(|_| internal_error())?
    else {
        return Err(not_found("warehouse_not_found"));
    };
    tx.rollback().await.ok();
    Ok(Json(row))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct WarehouseRequest {
    name: String,
    #[serde(default)]
    vat_rate_pct: BigDecimal,
    purchase_account_id: i64,
    purchase_return_account_id: i64,
    sales_account_id: i64,
    sales_return_account_id: i64,
    discount_account_id: i64,
    vat_account_id: i64,
    #[serde(default)]
    finished_goods_account_id: Option<i64>,
    #[serde(default)]
    raw_materials_account_id: Option<i64>,
    #[serde(default)]
    inventory_account_id: Option<i64>,
}

async fn validate_warehouse_accounts(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: i64,
    req: &WarehouseRequest,
) -> Result<(), (StatusCode, Json<Value>)> {
    require_leaf_account(tx, tenant_id, req.purchase_account_id, "purchase_account").await?;
    require_leaf_account(
        tx,
        tenant_id,
        req.purchase_return_account_id,
        "purchase_return_account",
    )
    .await?;
    require_leaf_account(tx, tenant_id, req.sales_account_id, "sales_account").await?;
    require_leaf_account(
        tx,
        tenant_id,
        req.sales_return_account_id,
        "sales_return_account",
    )
    .await?;
    require_leaf_account(tx, tenant_id, req.discount_account_id, "discount_account").await?;
    require_leaf_account(tx, tenant_id, req.vat_account_id, "vat_account").await?;
    if let Some(id) = req.finished_goods_account_id {
        require_leaf_account(tx, tenant_id, id, "finished_goods_account").await?;
    }
    if let Some(id) = req.raw_materials_account_id {
        require_leaf_account(tx, tenant_id, id, "raw_materials_account").await?;
    }
    if let Some(id) = req.inventory_account_id {
        require_leaf_account(tx, tenant_id, id, "inventory_account").await?;
    }
    Ok(())
}

async fn create_warehouse(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<WarehouseRequest>,
) -> Result<(StatusCode, Json<Value>), (StatusCode, Json<Value>)> {
    if req.name.trim().is_empty() {
        return Err(bad_request("invalid_name"));
    }
    let mut tx = db::begin(&state.pool, auth.tenant_id)
        .await
        .map_err(|_| internal_error())?;
    validate_warehouse_accounts(&mut tx, auth.tenant_id, &req).await?;

    let id: i64 = sqlx::query_scalar(
        "INSERT INTO warehouses \
         (tenant_id, name, vat_rate_pct, purchase_account_id, purchase_return_account_id, \
          sales_account_id, sales_return_account_id, discount_account_id, vat_account_id, \
          finished_goods_account_id, raw_materials_account_id, inventory_account_id, created_by) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13) RETURNING id",
    )
    .bind(auth.tenant_id)
    .bind(req.name.trim())
    .bind(&req.vat_rate_pct)
    .bind(req.purchase_account_id)
    .bind(req.purchase_return_account_id)
    .bind(req.sales_account_id)
    .bind(req.sales_return_account_id)
    .bind(req.discount_account_id)
    .bind(req.vat_account_id)
    .bind(req.finished_goods_account_id)
    .bind(req.raw_materials_account_id)
    .bind(req.inventory_account_id)
    .bind(auth.user_id)
    .fetch_one(&mut *tx)
    .await
    .map_err(|_| internal_error())?;

    audit::record_mutation(
        &mut tx,
        auth.tenant_id,
        "warehouses",
        id,
        "insert",
        Some(auth.user_id),
        None,
        Some(json!({ "name": req.name.trim() })),
    )
    .await
    .map_err(|_| internal_error())?;

    tx.commit().await.map_err(|_| internal_error())?;
    Ok((StatusCode::CREATED, Json(json!({ "id": id }))))
}

async fn update_warehouse(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
    Json(req): Json<WarehouseRequest>,
) -> Result<StatusCode, (StatusCode, Json<Value>)> {
    if req.name.trim().is_empty() {
        return Err(bad_request("invalid_name"));
    }
    let mut tx = db::begin(&state.pool, auth.tenant_id)
        .await
        .map_err(|_| internal_error())?;
    let Some(existing) = fetch_warehouse(&mut tx, auth.tenant_id, id)
        .await
        .map_err(|_| internal_error())?
    else {
        return Err(not_found("warehouse_not_found"));
    };
    validate_warehouse_accounts(&mut tx, auth.tenant_id, &req).await?;

    sqlx::query(
        "UPDATE warehouses SET name = $1, vat_rate_pct = $2, purchase_account_id = $3, \
         purchase_return_account_id = $4, sales_account_id = $5, sales_return_account_id = $6, \
         discount_account_id = $7, vat_account_id = $8, finished_goods_account_id = $9, \
         raw_materials_account_id = $10, inventory_account_id = $11, updated_at = now(), \
         updated_by = $12 WHERE id = $13",
    )
    .bind(req.name.trim())
    .bind(&req.vat_rate_pct)
    .bind(req.purchase_account_id)
    .bind(req.purchase_return_account_id)
    .bind(req.sales_account_id)
    .bind(req.sales_return_account_id)
    .bind(req.discount_account_id)
    .bind(req.vat_account_id)
    .bind(req.finished_goods_account_id)
    .bind(req.raw_materials_account_id)
    .bind(req.inventory_account_id)
    .bind(auth.user_id)
    .bind(id)
    .execute(&mut *tx)
    .await
    .map_err(|_| internal_error())?;

    audit::record_mutation(
        &mut tx,
        auth.tenant_id,
        "warehouses",
        id,
        "update",
        Some(auth.user_id),
        Some(json!({ "name": existing.name })),
        Some(json!({ "name": req.name.trim() })),
    )
    .await
    .map_err(|_| internal_error())?;

    tx.commit().await.map_err(|_| internal_error())?;
    Ok(StatusCode::NO_CONTENT)
}

/// §1.1's real fix for the legacy's never-implemented "N2" delete handler: a warehouse can be
/// deactivated once it holds no item assignments. Not a hard delete (nothing in the Build bullet
/// asks for one, and once 5.2 lands a warehouse will carry document history too).
async fn deactivate_warehouse(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<StatusCode, (StatusCode, Json<Value>)> {
    let mut tx = db::begin(&state.pool, auth.tenant_id)
        .await
        .map_err(|_| internal_error())?;
    let Some(_) = fetch_warehouse(&mut tx, auth.tenant_id, id)
        .await
        .map_err(|_| internal_error())?
    else {
        return Err(not_found("warehouse_not_found"));
    };
    let item_count: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM item_warehouses WHERE tenant_id = $1 AND warehouse_id = $2",
    )
    .bind(auth.tenant_id)
    .bind(id)
    .fetch_one(&mut *tx)
    .await
    .map_err(|_| internal_error())?;
    if item_count > 0 {
        return Err(bad_request("warehouse_not_empty"));
    }

    sqlx::query("UPDATE warehouses SET is_active = false, updated_at = now(), updated_by = $1 WHERE id = $2")
        .bind(auth.user_id)
        .bind(id)
        .execute(&mut *tx)
        .await
        .map_err(|_| internal_error())?;

    audit::record_mutation(
        &mut tx,
        auth.tenant_id,
        "warehouses",
        id,
        "update",
        Some(auth.user_id),
        Some(json!({ "isActive": true })),
        Some(json!({ "isActive": false })),
    )
    .await
    .map_err(|_| internal_error())?;

    tx.commit().await.map_err(|_| internal_error())?;
    Ok(StatusCode::NO_CONTENT)
}

async fn activate_warehouse(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<StatusCode, (StatusCode, Json<Value>)> {
    let mut tx = db::begin(&state.pool, auth.tenant_id)
        .await
        .map_err(|_| internal_error())?;
    let Some(_) = fetch_warehouse(&mut tx, auth.tenant_id, id)
        .await
        .map_err(|_| internal_error())?
    else {
        return Err(not_found("warehouse_not_found"));
    };
    sqlx::query(
        "UPDATE warehouses SET is_active = true, updated_at = now(), updated_by = $1 WHERE id = $2",
    )
    .bind(auth.user_id)
    .bind(id)
    .execute(&mut *tx)
    .await
    .map_err(|_| internal_error())?;

    audit::record_mutation(
        &mut tx,
        auth.tenant_id,
        "warehouses",
        id,
        "update",
        Some(auth.user_id),
        Some(json!({ "isActive": false })),
        Some(json!({ "isActive": true })),
    )
    .await
    .map_err(|_| internal_error())?;

    tx.commit().await.map_err(|_| internal_error())?;
    Ok(StatusCode::NO_CONTENT)
}

// =========================================================================
// units_of_measure   <- legacy Anbar_Vahed (§1.3) — no legacy maintenance screen at all
// =========================================================================

pub fn units_of_measure_router() -> Router<AppState> {
    Router::new()
        .route("/", get(list_units).post(create_unit))
        .route("/{id}", put(update_unit))
}

#[derive(sqlx::FromRow, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct UnitRecord {
    id: i64,
    name: String,
    base_unit_id: Option<i64>,
    conversion_factor: BigDecimal,
}

const UNIT_COLUMNS: &str = "id, name, base_unit_id, conversion_factor";

async fn list_units(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Vec<UnitRecord>>, (StatusCode, Json<Value>)> {
    let mut tx = db::begin(&state.pool, auth.tenant_id)
        .await
        .map_err(|_| internal_error())?;
    let rows = sqlx::query_as(&format!(
        "SELECT {UNIT_COLUMNS} FROM units_of_measure WHERE tenant_id = $1 ORDER BY name"
    ))
    .bind(auth.tenant_id)
    .fetch_all(&mut *tx)
    .await
    .map_err(|_| internal_error())?;
    tx.rollback().await.ok();
    Ok(Json(rows))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct UnitRequest {
    name: String,
    #[serde(default)]
    base_unit_id: Option<i64>,
    #[serde(default)]
    conversion_factor: Option<BigDecimal>,
}

/// Single-level base-unit resolution only (a judgment call, documented in the migration's own
/// comment) — the referenced base must itself be a base unit (no `base_unit_id` of its own), so
/// there is never a chain to walk.
async fn validate_unit_request(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: i64,
    self_id: Option<i64>,
    req: &UnitRequest,
) -> Result<BigDecimal, (StatusCode, Json<Value>)> {
    match req.base_unit_id {
        None => Ok(BigDecimal::from(1)),
        Some(base_id) => {
            if Some(base_id) == self_id {
                return Err(bad_request("unit_cannot_be_own_base"));
            }
            let base: Option<(Option<i64>,)> = sqlx::query_as(
                "SELECT base_unit_id FROM units_of_measure WHERE tenant_id = $1 AND id = $2",
            )
            .bind(tenant_id)
            .bind(base_id)
            .fetch_optional(&mut **tx)
            .await
            .map_err(|_| internal_error())?;
            match base {
                None => Err(bad_request("base_unit_not_found")),
                Some((Some(_),)) => Err(bad_request("base_unit_must_not_itself_be_derived")),
                Some((None,)) => {
                    let factor = req
                        .conversion_factor
                        .clone()
                        .unwrap_or_else(|| BigDecimal::from(1));
                    if factor.sign() != bigdecimal::num_bigint::Sign::Plus {
                        return Err(bad_request("invalid_conversion_factor"));
                    }
                    Ok(factor)
                }
            }
        }
    }
}

async fn create_unit(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<UnitRequest>,
) -> Result<(StatusCode, Json<Value>), (StatusCode, Json<Value>)> {
    if req.name.trim().is_empty() {
        return Err(bad_request("invalid_name"));
    }
    let mut tx = db::begin(&state.pool, auth.tenant_id)
        .await
        .map_err(|_| internal_error())?;
    let factor = validate_unit_request(&mut tx, auth.tenant_id, None, &req).await?;

    let id: i64 = sqlx::query_scalar(
        "INSERT INTO units_of_measure (tenant_id, name, base_unit_id, conversion_factor) \
         VALUES ($1, $2, $3, $4) RETURNING id",
    )
    .bind(auth.tenant_id)
    .bind(req.name.trim())
    .bind(req.base_unit_id)
    .bind(&factor)
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| map_unique_violation(e, "units_of_measure_name_key", "duplicate_name"))?;

    audit::record_mutation(
        &mut tx,
        auth.tenant_id,
        "units_of_measure",
        id,
        "insert",
        Some(auth.user_id),
        None,
        Some(json!({ "name": req.name.trim() })),
    )
    .await
    .map_err(|_| internal_error())?;

    tx.commit().await.map_err(|_| internal_error())?;
    Ok((StatusCode::CREATED, Json(json!({ "id": id }))))
}

async fn update_unit(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
    Json(req): Json<UnitRequest>,
) -> Result<StatusCode, (StatusCode, Json<Value>)> {
    if req.name.trim().is_empty() {
        return Err(bad_request("invalid_name"));
    }
    let mut tx = db::begin(&state.pool, auth.tenant_id)
        .await
        .map_err(|_| internal_error())?;
    let existing: Option<(String,)> =
        sqlx::query_as("SELECT name FROM units_of_measure WHERE tenant_id = $1 AND id = $2")
            .bind(auth.tenant_id)
            .bind(id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(|_| internal_error())?;
    let Some((old_name,)) = existing else {
        return Err(not_found("unit_of_measure_not_found"));
    };
    let factor = validate_unit_request(&mut tx, auth.tenant_id, Some(id), &req).await?;

    sqlx::query(
        "UPDATE units_of_measure SET name = $1, base_unit_id = $2, conversion_factor = $3 WHERE id = $4",
    )
    .bind(req.name.trim())
    .bind(req.base_unit_id)
    .bind(&factor)
    .bind(id)
    .execute(&mut *tx)
    .await
    .map_err(|e| map_unique_violation(e, "units_of_measure_name_key", "duplicate_name"))?;

    audit::record_mutation(
        &mut tx,
        auth.tenant_id,
        "units_of_measure",
        id,
        "update",
        Some(auth.user_id),
        Some(json!({ "name": old_name })),
        Some(json!({ "name": req.name.trim() })),
    )
    .await
    .map_err(|_| internal_error())?;

    tx.commit().await.map_err(|_| internal_error())?;
    Ok(StatusCode::NO_CONTENT)
}

// =========================================================================
// pistachio_grades   <- legacy Kinds (§1.4/§8.1) — fixed 7-value enumeration, no user CRUD
// =========================================================================

pub fn pistachio_grades_router() -> Router<AppState> {
    Router::new()
        .route("/", get(list_pistachio_grades))
        .route("/seed-defaults", post(seed_pistachio_grades))
}

#[derive(sqlx::FromRow, Serialize)]
#[serde(rename_all = "camelCase")]
struct GradeRecord {
    id: i64,
    name: String,
    sort_order: i32,
}

async fn list_pistachio_grades(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Vec<GradeRecord>>, (StatusCode, Json<Value>)> {
    let mut tx = db::begin(&state.pool, auth.tenant_id)
        .await
        .map_err(|_| internal_error())?;
    let rows = sqlx::query_as(
        "SELECT id, name, sort_order FROM pistachio_grades WHERE tenant_id = $1 ORDER BY sort_order",
    )
    .bind(auth.tenant_id)
    .fetch_all(&mut *tx)
    .await
    .map_err(|_| internal_error())?;
    tx.rollback().await.ok();
    Ok(Json(rows))
}

/// 05-08-a.md §8.1's seven-value enumeration, recovered from a source comment
/// (`FactorPesteh_U.pas:132-133`) — the only place it exists in the legacy at all.
const DEFAULT_GRADES: [(i32, &str); 7] = [
    (1, "Fandoghi"),
    (2, "Badami"),
    (3, "Kalleh-Ghouchi"),
    (4, "Momtaz"),
    (5, "Ahmad-Aghaei"),
    (6, "Akbari"),
    (7, "Dahan-Bast"),
];

/// Same "no tenant-provisioning flow exists yet" gap 2.1/3.1 documented for
/// `account_code_format`/`party_account_config` — a superuser-triggered, idempotent seed rather
/// than baking the 7 rows into the migration for tenants that don't exist yet.
async fn seed_pistachio_grades(
    State(state): State<AppState>,
    admin: RequireSuperuser,
) -> Result<StatusCode, (StatusCode, Json<Value>)> {
    let auth = admin.0;
    let mut tx = db::begin(&state.pool, auth.tenant_id)
        .await
        .map_err(|_| internal_error())?;
    for (sort_order, name) in DEFAULT_GRADES {
        sqlx::query(
            "INSERT INTO pistachio_grades (tenant_id, name, sort_order) VALUES ($1, $2, $3) \
             ON CONFLICT (tenant_id, sort_order) DO NOTHING",
        )
        .bind(auth.tenant_id)
        .bind(name)
        .bind(sort_order)
        .execute(&mut *tx)
        .await
        .map_err(|_| internal_error())?;
    }
    tx.commit().await.map_err(|_| internal_error())?;
    Ok(StatusCode::NO_CONTENT)
}

// =========================================================================
// items   <- merges legacy Anbar_Jens (subsystem A) + Cala (subsystem B), §1.6
// =========================================================================

pub fn items_router() -> Router<AppState> {
    Router::new()
        .route("/", get(list_items).post(create_item))
        .route("/{id}", get(get_item).put(update_item).delete(delete_item))
        .route("/{id}/activate", post(activate_item))
        .route("/{id}/deactivate", post(deactivate_item))
        .route("/{id}/warehouses", post(assign_warehouse))
        .route(
            "/{id}/warehouses/{warehouse_id}",
            axum::routing::delete(unassign_warehouse),
        )
        .route("/{id}/on-hand", get(get_on_hand))
        .route("/{id}/stock-card", get(get_stock_card))
        .route("/{id}/average-cost", get(get_average_cost))
}

#[derive(sqlx::FromRow, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct ItemRecord {
    id: i64,
    code: i32,
    name: String,
    specification: Option<String>,
    unit_of_measure_id: i64,
    sale_price: i64,
    min_stock: i64,
    is_taxable: bool,
    allow_negative_stock: bool,
    tax_item_code: Option<String>,
    is_active: bool,
    pistachio_grade_id: Option<i64>,
}

const ITEM_COLUMNS: &str = "id, code, name, specification, unit_of_measure_id, sale_price, \
    min_stock, is_taxable, allow_negative_stock, tax_item_code, is_active, pistachio_grade_id";

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ItemDetail {
    #[serde(flatten)]
    item: ItemRecord,
    warehouse_ids: Vec<i64>,
}

async fn fetch_item(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: i64,
    id: i64,
) -> Result<Option<ItemRecord>, sqlx::Error> {
    sqlx::query_as(&format!(
        "SELECT {ITEM_COLUMNS} FROM items WHERE tenant_id = $1 AND id = $2"
    ))
    .bind(tenant_id)
    .bind(id)
    .fetch_optional(&mut **tx)
    .await
}

async fn fetch_item_warehouse_ids(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: i64,
    item_id: i64,
) -> Result<Vec<i64>, sqlx::Error> {
    sqlx::query_scalar(
        "SELECT warehouse_id FROM item_warehouses WHERE tenant_id = $1 AND item_id = $2 ORDER BY warehouse_id",
    )
    .bind(tenant_id)
    .bind(item_id)
    .fetch_all(&mut **tx)
    .await
}

#[derive(Deserialize)]
struct ListItemsQuery {
    #[serde(rename = "warehouseId")]
    warehouse_id: Option<i64>,
    search: Option<String>,
    #[serde(default, rename = "activeOnly")]
    active_only: bool,
}

/// Item search, replacing `AnbarCalaSelectU`'s `PATINDEX`-on-every-keystroke query (§2.6) — a
/// plain case-insensitive substring match with no 18-character truncation and no wildcard
/// injection from user input (the term is always bound as a literal, `%`/`_` included).
async fn list_items(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(q): Query<ListItemsQuery>,
) -> Result<Json<Vec<ItemRecord>>, (StatusCode, Json<Value>)> {
    let mut tx = db::begin(&state.pool, auth.tenant_id)
        .await
        .map_err(|_| internal_error())?;

    let rows = if let Some(warehouse_id) = q.warehouse_id {
        sqlx::query_as(&format!(
            "SELECT {cols} FROM items i \
             JOIN item_warehouses iw ON iw.item_id = i.id AND iw.tenant_id = i.tenant_id \
             WHERE i.tenant_id = $1 AND iw.warehouse_id = $2 \
             AND ($3::text IS NULL OR i.name ILIKE '%' || $3 || '%') \
             AND ($4 = false OR i.is_active) \
             ORDER BY i.name",
            cols = ITEM_COLUMNS
                .split(", ")
                .map(|c| format!("i.{c}"))
                .collect::<Vec<_>>()
                .join(", ")
        ))
        .bind(auth.tenant_id)
        .bind(warehouse_id)
        .bind(&q.search)
        .bind(q.active_only)
        .fetch_all(&mut *tx)
        .await
        .map_err(|_| internal_error())?
    } else {
        sqlx::query_as(&format!(
            "SELECT {ITEM_COLUMNS} FROM items \
             WHERE tenant_id = $1 AND ($2::text IS NULL OR name ILIKE '%' || $2 || '%') \
             AND ($3 = false OR is_active) ORDER BY name"
        ))
        .bind(auth.tenant_id)
        .bind(&q.search)
        .bind(q.active_only)
        .fetch_all(&mut *tx)
        .await
        .map_err(|_| internal_error())?
    };
    tx.rollback().await.ok();
    Ok(Json(rows))
}

async fn get_item(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<Json<ItemDetail>, (StatusCode, Json<Value>)> {
    let mut tx = db::begin(&state.pool, auth.tenant_id)
        .await
        .map_err(|_| internal_error())?;
    let Some(item) = fetch_item(&mut tx, auth.tenant_id, id)
        .await
        .map_err(|_| internal_error())?
    else {
        return Err(not_found("item_not_found"));
    };
    let warehouse_ids = fetch_item_warehouse_ids(&mut tx, auth.tenant_id, id)
        .await
        .map_err(|_| internal_error())?;
    tx.rollback().await.ok();
    Ok(Json(ItemDetail {
        item,
        warehouse_ids,
    }))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ItemWriteRequest {
    name: String,
    #[serde(default)]
    specification: Option<String>,
    unit_of_measure_id: i64,
    sale_price: i64,
    #[serde(default)]
    min_stock: i64,
    #[serde(default)]
    is_taxable: bool,
    #[serde(default = "default_true")]
    allow_negative_stock: bool,
    #[serde(default)]
    tax_item_code: Option<String>,
    #[serde(default)]
    pistachio_grade_id: Option<i64>,
}
fn default_true() -> bool {
    true // AJ_Manfi's legacy default (§2.2.2) — "negative stock permitted" out of the box.
}

/// §2.2's four validations, in order, minus the duplicate-code check (handled separately at
/// create only — code is immutable on update, §2.1/§2.3). `sale_price <> 0` only, negative
/// allowed — [AS-IS] per §2.7's explicit "default is port-as-is".
async fn validate_item_request(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: i64,
    req: &ItemWriteRequest,
) -> Result<(), (StatusCode, Json<Value>)> {
    if req.name.trim().is_empty() {
        return Err(bad_request("invalid_name"));
    }
    let uom_exists: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM units_of_measure WHERE tenant_id = $1 AND id = $2)",
    )
    .bind(tenant_id)
    .bind(req.unit_of_measure_id)
    .fetch_one(&mut **tx)
    .await
    .map_err(|_| internal_error())?;
    if !uom_exists {
        return Err(bad_request("unit_of_measure_not_found"));
    }
    if req.sale_price == 0 {
        return Err(bad_request("sale_price_required"));
    }
    if let Some(grade_id) = req.pistachio_grade_id {
        let grade_exists: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM pistachio_grades WHERE tenant_id = $1 AND id = $2)",
        )
        .bind(tenant_id)
        .bind(grade_id)
        .fetch_one(&mut **tx)
        .await
        .map_err(|_| internal_error())?;
        if !grade_exists {
            return Err(bad_request("pistachio_grade_not_found"));
        }
    }
    Ok(())
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateItemRequest {
    code: i32,
    #[serde(flatten)]
    fields: ItemWriteRequest,
    #[serde(default)]
    warehouse_ids: Vec<i64>,
}

async fn create_item(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<CreateItemRequest>,
) -> Result<(StatusCode, Json<Value>), (StatusCode, Json<Value>)> {
    let mut tx = db::begin(&state.pool, auth.tenant_id)
        .await
        .map_err(|_| internal_error())?;
    validate_item_request(&mut tx, auth.tenant_id, &req.fields).await?;

    for warehouse_id in &req.warehouse_ids {
        let exists: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM warehouses WHERE tenant_id = $1 AND id = $2)",
        )
        .bind(auth.tenant_id)
        .bind(warehouse_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(|_| internal_error())?;
        if !exists {
            return Err(bad_request("warehouse_not_found"));
        }
    }

    let id: i64 = sqlx::query_scalar(
        "INSERT INTO items \
         (tenant_id, code, name, specification, unit_of_measure_id, sale_price, min_stock, \
          is_taxable, allow_negative_stock, tax_item_code, pistachio_grade_id, created_by) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12) RETURNING id",
    )
    .bind(auth.tenant_id)
    .bind(req.code)
    .bind(req.fields.name.trim())
    .bind(req.fields.specification.as_deref().map(str::trim))
    .bind(req.fields.unit_of_measure_id)
    .bind(req.fields.sale_price)
    .bind(req.fields.min_stock)
    .bind(req.fields.is_taxable)
    .bind(req.fields.allow_negative_stock)
    .bind(req.fields.tax_item_code.as_deref().map(str::trim))
    .bind(req.fields.pistachio_grade_id)
    .bind(auth.user_id)
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| map_unique_violation(e, "items_code_key", "duplicate_code"))?;

    for warehouse_id in &req.warehouse_ids {
        sqlx::query(
            "INSERT INTO item_warehouses (tenant_id, item_id, warehouse_id) VALUES ($1, $2, $3)",
        )
        .bind(auth.tenant_id)
        .bind(id)
        .bind(warehouse_id)
        .execute(&mut *tx)
        .await
        .map_err(|_| internal_error())?;
    }

    audit::record_mutation(
        &mut tx,
        auth.tenant_id,
        "items",
        id,
        "insert",
        Some(auth.user_id),
        None,
        Some(json!({ "code": req.code, "name": req.fields.name.trim() })),
    )
    .await
    .map_err(|_| internal_error())?;

    tx.commit().await.map_err(|_| internal_error())?;
    Ok((StatusCode::CREATED, Json(json!({ "id": id }))))
}

/// Code is never in the update column list — immutable after create, matching §2.3's exact
/// legacy behaviour ("`AJ_Code` is not in the `UPDATE` list"). Unlike the legacy, this can never
/// produce an unmaintainable item (§2.1's "code 0" hazard) because edit is always by the real
/// surrogate `id`, never by re-locating a client-side cache on the code.
async fn update_item(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
    Json(req): Json<ItemWriteRequest>,
) -> Result<StatusCode, (StatusCode, Json<Value>)> {
    let mut tx = db::begin(&state.pool, auth.tenant_id)
        .await
        .map_err(|_| internal_error())?;
    let Some(existing) = fetch_item(&mut tx, auth.tenant_id, id)
        .await
        .map_err(|_| internal_error())?
    else {
        return Err(not_found("item_not_found"));
    };
    validate_item_request(&mut tx, auth.tenant_id, &req).await?;

    sqlx::query(
        "UPDATE items SET name = $1, specification = $2, unit_of_measure_id = $3, sale_price = $4, \
         min_stock = $5, is_taxable = $6, allow_negative_stock = $7, tax_item_code = $8, \
         pistachio_grade_id = $9, updated_at = now(), updated_by = $10 WHERE id = $11",
    )
    .bind(req.name.trim())
    .bind(req.specification.as_deref().map(str::trim))
    .bind(req.unit_of_measure_id)
    .bind(req.sale_price)
    .bind(req.min_stock)
    .bind(req.is_taxable)
    .bind(req.allow_negative_stock)
    .bind(req.tax_item_code.as_deref().map(str::trim))
    .bind(req.pistachio_grade_id)
    .bind(auth.user_id)
    .bind(id)
    .execute(&mut *tx)
    .await
    .map_err(|_| internal_error())?;

    audit::record_mutation(
        &mut tx,
        auth.tenant_id,
        "items",
        id,
        "update",
        Some(auth.user_id),
        Some(json!({ "name": existing.name, "salePrice": existing.sale_price })),
        Some(json!({ "name": req.name.trim(), "salePrice": req.sale_price })),
    )
    .await
    .map_err(|_| internal_error())?;

    tx.commit().await.map_err(|_| internal_error())?;
    Ok(StatusCode::NO_CONTENT)
}

async fn set_item_active(
    state: &AppState,
    auth: &AuthUser,
    id: i64,
    active: bool,
) -> Result<StatusCode, (StatusCode, Json<Value>)> {
    let mut tx = db::begin(&state.pool, auth.tenant_id)
        .await
        .map_err(|_| internal_error())?;
    let Some(existing) = fetch_item(&mut tx, auth.tenant_id, id)
        .await
        .map_err(|_| internal_error())?
    else {
        return Err(not_found("item_not_found"));
    };
    sqlx::query(
        "UPDATE items SET is_active = $1, updated_at = now(), updated_by = $2 WHERE id = $3",
    )
    .bind(active)
    .bind(auth.user_id)
    .bind(id)
    .execute(&mut *tx)
    .await
    .map_err(|_| internal_error())?;

    audit::record_mutation(
        &mut tx,
        auth.tenant_id,
        "items",
        id,
        "update",
        Some(auth.user_id),
        Some(json!({ "isActive": existing.is_active })),
        Some(json!({ "isActive": active })),
    )
    .await
    .map_err(|_| internal_error())?;

    tx.commit().await.map_err(|_| internal_error())?;
    Ok(StatusCode::NO_CONTENT)
}

/// The real retirement path §1.2/§2.4 flags as entirely missing from the legacy ("there is
/// therefore no way to retire a used item at all") — an alternative to `delete_item` for an item
/// that has ever been invoiced and so can never be hard-deleted.
async fn deactivate_item(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<StatusCode, (StatusCode, Json<Value>)> {
    set_item_active(&state, &auth, id, false).await
}

async fn activate_item(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<StatusCode, (StatusCode, Json<Value>)> {
    set_item_active(&state, &auth, id, true).await
}

/// §2.4's usage guard ("با اين کد فاکتور صادر شده است" — an invoice has been issued with this
/// code) is a stub here, the same documented gap 2.1 left for `has_postings` — the table it
/// would check, `inventory_document_lines`, doesn't exist until step 5.2. Unlike the legacy,
/// this delete DOES require a confirmation step at the UI layer (5.9) — §2.4 finding 1 flags the
/// legacy's single-click, no-confirmation delete as a real gap, not behaviour to preserve.
async fn delete_item(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<StatusCode, (StatusCode, Json<Value>)> {
    let mut tx = db::begin(&state.pool, auth.tenant_id)
        .await
        .map_err(|_| internal_error())?;
    let Some(existing) = fetch_item(&mut tx, auth.tenant_id, id)
        .await
        .map_err(|_| internal_error())?
    else {
        return Err(not_found("item_not_found"));
    };

    sqlx::query("DELETE FROM item_warehouses WHERE tenant_id = $1 AND item_id = $2")
        .bind(auth.tenant_id)
        .bind(id)
        .execute(&mut *tx)
        .await
        .map_err(|_| internal_error())?;
    sqlx::query("DELETE FROM items WHERE id = $1")
        .bind(id)
        .execute(&mut *tx)
        .await
        .map_err(|_| internal_error())?;

    audit::record_mutation(
        &mut tx,
        auth.tenant_id,
        "items",
        id,
        "delete",
        Some(auth.user_id),
        Some(json!({ "code": existing.code, "name": existing.name })),
        None,
    )
    .await
    .map_err(|_| internal_error())?;

    tx.commit().await.map_err(|_| internal_error())?;
    Ok(StatusCode::NO_CONTENT)
}

// ---- item <-> warehouse assignment (§1.6's real junction, replacing AJ_ID / C_Anbar) ----

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct AssignWarehouseRequest {
    warehouse_id: i64,
}

async fn assign_warehouse(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
    Json(req): Json<AssignWarehouseRequest>,
) -> Result<StatusCode, (StatusCode, Json<Value>)> {
    let mut tx = db::begin(&state.pool, auth.tenant_id)
        .await
        .map_err(|_| internal_error())?;
    let Some(_) = fetch_item(&mut tx, auth.tenant_id, id)
        .await
        .map_err(|_| internal_error())?
    else {
        return Err(not_found("item_not_found"));
    };
    let warehouse_exists: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM warehouses WHERE tenant_id = $1 AND id = $2)",
    )
    .bind(auth.tenant_id)
    .bind(req.warehouse_id)
    .fetch_one(&mut *tx)
    .await
    .map_err(|_| internal_error())?;
    if !warehouse_exists {
        return Err(bad_request("warehouse_not_found"));
    }

    sqlx::query(
        "INSERT INTO item_warehouses (tenant_id, item_id, warehouse_id) VALUES ($1, $2, $3) \
         ON CONFLICT (item_id, warehouse_id) DO NOTHING",
    )
    .bind(auth.tenant_id)
    .bind(id)
    .bind(req.warehouse_id)
    .execute(&mut *tx)
    .await
    .map_err(|_| internal_error())?;

    audit::record_mutation(
        &mut tx,
        auth.tenant_id,
        "item_warehouses",
        id,
        "insert",
        Some(auth.user_id),
        None,
        Some(json!({ "itemId": id, "warehouseId": req.warehouse_id })),
    )
    .await
    .map_err(|_| internal_error())?;

    tx.commit().await.map_err(|_| internal_error())?;
    Ok(StatusCode::NO_CONTENT)
}

// ---- on-hand / stock card (step 5.3) ------------------------------------
//
// Both handlers are thin — the real logic is `stock::compute_on_hand`/`compute_stock_card`,
// the one canonical balance function §11.4 requirement #1 asks for. `fiscalYearId`/`asOfDate`
// are required query params, not the session's implicit current year/today — the legacy's own
// Card Jensi screen is "the only inventory screen where the fiscal year is selectable" (§11.1.1),
// and Mandeh's hard-wiring to the session year is flagged as the worse of the two designs.

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct OnHandQuery {
    fiscal_year_id: i64,
    as_of_date: NaiveDate,
    #[serde(default)]
    warehouse_id: Option<i64>,
    #[serde(default)]
    exclude_document_id: Option<i64>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct OnHandResponse {
    on_hand: BigDecimal,
    min_stock: i64,
    /// The real low-stock alert the legacy never had (§1.2's "displayed, never checked" finding,
    /// deferred by 5.1's own manual test #4 until this step's on-hand query existed) — a genuine
    /// comparison, not a passive number next to the balance.
    is_low_stock: bool,
}

async fn get_on_hand(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
    Query(q): Query<OnHandQuery>,
) -> Result<Json<OnHandResponse>, (StatusCode, Json<Value>)> {
    let mut tx = db::begin(&state.pool, auth.tenant_id)
        .await
        .map_err(|_| internal_error())?;
    let Some(item) = fetch_item(&mut tx, auth.tenant_id, id)
        .await
        .map_err(|_| internal_error())?
    else {
        return Err(not_found("item_not_found"));
    };
    let on_hand = crate::stock::compute_on_hand(
        &mut tx,
        auth.tenant_id,
        id,
        q.fiscal_year_id,
        q.warehouse_id,
        q.as_of_date,
        q.exclude_document_id,
    )
    .await
    .map_err(|_| internal_error())?;
    tx.rollback().await.ok();

    #[allow(clippy::cmp_owned)] // comparing a numeric(14,3) on-hand figure against an i64 threshold
    let is_low_stock = on_hand < BigDecimal::from(item.min_stock);
    Ok(Json(OnHandResponse {
        on_hand,
        min_stock: item.min_stock,
        is_low_stock,
    }))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct StockCardQuery {
    fiscal_year_id: i64,
    from_date: NaiveDate,
    to_date: NaiveDate,
    #[serde(default)]
    warehouse_id: Option<i64>,
}

async fn get_stock_card(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
    Query(q): Query<StockCardQuery>,
) -> Result<Json<crate::stock::StockCard>, (StatusCode, Json<Value>)> {
    if q.from_date > q.to_date {
        return Err(bad_request("invalid_date_range"));
    }
    let mut tx = db::begin(&state.pool, auth.tenant_id)
        .await
        .map_err(|_| internal_error())?;
    let Some(_) = fetch_item(&mut tx, auth.tenant_id, id)
        .await
        .map_err(|_| internal_error())?
    else {
        return Err(not_found("item_not_found"));
    };
    let card = crate::stock::compute_stock_card(
        &mut tx,
        auth.tenant_id,
        id,
        q.fiscal_year_id,
        q.warehouse_id,
        q.from_date,
        q.to_date,
    )
    .await
    .map_err(|_| internal_error())?;
    tx.rollback().await.ok();
    Ok(Json(card))
}

/// Step 5.4 (docs/phase-5-inventory.md §5.4): the weighted-average-of-purchases suggestion.
/// Query params mirror `get_on_hand`'s exactly, same reasoning — an explicit fiscal year/date, not
/// the session's implicit current one, and the same document being edited can exclude itself.
async fn get_average_cost(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
    Query(q): Query<OnHandQuery>,
) -> Result<Json<crate::stock::AverageCost>, (StatusCode, Json<Value>)> {
    let mut tx = db::begin(&state.pool, auth.tenant_id)
        .await
        .map_err(|_| internal_error())?;
    let Some(_) = fetch_item(&mut tx, auth.tenant_id, id)
        .await
        .map_err(|_| internal_error())?
    else {
        return Err(not_found("item_not_found"));
    };
    let cost = crate::stock::compute_average_cost(
        &mut tx,
        auth.tenant_id,
        id,
        q.fiscal_year_id,
        q.warehouse_id,
        q.as_of_date,
        q.exclude_document_id,
    )
    .await
    .map_err(|_| internal_error())?;
    tx.rollback().await.ok();
    Ok(Json(cost))
}

async fn unassign_warehouse(
    State(state): State<AppState>,
    auth: AuthUser,
    Path((id, warehouse_id)): Path<(i64, i64)>,
) -> Result<StatusCode, (StatusCode, Json<Value>)> {
    let mut tx = db::begin(&state.pool, auth.tenant_id)
        .await
        .map_err(|_| internal_error())?;
    let result = sqlx::query(
        "DELETE FROM item_warehouses WHERE tenant_id = $1 AND item_id = $2 AND warehouse_id = $3",
    )
    .bind(auth.tenant_id)
    .bind(id)
    .bind(warehouse_id)
    .execute(&mut *tx)
    .await
    .map_err(|_| internal_error())?;
    if result.rows_affected() == 0 {
        return Err(not_found("assignment_not_found"));
    }

    audit::record_mutation(
        &mut tx,
        auth.tenant_id,
        "item_warehouses",
        id,
        "delete",
        Some(auth.user_id),
        Some(json!({ "itemId": id, "warehouseId": warehouse_id })),
        None,
    )
    .await
    .map_err(|_| internal_error())?;

    tx.commit().await.map_err(|_| internal_error())?;
    Ok(StatusCode::NO_CONTENT)
}
