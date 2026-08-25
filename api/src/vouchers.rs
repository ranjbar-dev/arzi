//! Step 2.3 (docs/phase-2-accounting-core.md §2.3): vouchers + voucher lines,
//! the double-entry engine everything else in the system eventually posts
//! through. specs/03-accounting-core/03-03-a/b/c-voucher-sanad-model.md and
//! 03-04-voucher-validation-rules.md are the behavioural ground truth.
//!
//! Departs from the legacy's "one grid, replace-all-manual-lines-on-save"
//! editing model (SanadEditU.pas:618-634) in favour of incremental per-line
//! CRUD, matching this codebase's existing resource-per-endpoint shape
//! (accounts.rs, admin.rs) — the guards (draft-only editing, immutable
//! generated lines, leaf-only accounts) are preserved exactly, just applied
//! per mutation instead of per whole-grid replace.
//!
//! Header totals (`total_debit`/`total_credit`/`line_count`) are maintained
//! incrementally, in the same transaction as every line insert/update/
//! delete — never the legacy's out-of-transaction `Dmoein_UpdateMab`
//! recompute (C3, "derive, don't store" — this is the in-transaction
//! equivalent of deriving them).
//!
//! The fiscal-year-open gate isn't a Pass_Config permission at all — it's
//! `fiscal_years.is_active`, already modelled in 1.5, applied here to every
//! mutating action (03-03-c.md §3.8's `Is_New_Sanad_Valid` gate). Judgment
//! call: the legacy applies this gate inconsistently (missing from the 1->2
//! transition, "probably an oversight" per §3.6) — the rebuild applies it
//! uniformly to every transition rather than porting the gap.
//!
//! Step 2.8 (docs/phase-2-accounting-core.md §2.8): every route below is now
//! permission-gated too, via plain runtime `AuthUser::has_permission` checks
//! rather than accounts.rs's static `RequirePermission<P>` — the catalogue id
//! for the mutating actions below (03-13-permissions.md §13.2, the
//! `sRollOutPanel2` subsidiary-document group) is only known after the
//! voucher is fetched, which the type-level extractor can't express.
//! `list`/`get` require any ONE of the three status-scoped view ids
//! (1125/1126/1127) since one unified endpoint replaces the legacy's three
//! separate list screens by status.

use crate::{audit, auth::AuthUser, db, AppState};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post, put},
    Json, Router,
};
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::{Postgres, Transaction};

/// 03-13-permissions.md §13.2 codes this module gates on, split by `kind`
/// where the catalogue keeps the two document families separate.
mod permcodes {
    pub const LIST_VIEW: &[&str] = &[
        "list_draft_subsidiary_documents",    // 1127
        "list_approved_subsidiary_documents", // 1125
        "list_posted_subsidiary_documents",   // 1126
    ];
    pub const POST_LEDGER: &str = "post_subsidiary_document"; // 1113
    pub const AMEND_LINE: &str = "amend_subsidiary_document"; // 1114

    /// (base "amend", date-change, number-change) codes for `update_voucher`.
    pub const HEADER_EDIT: (&str, &str, &str) = (
        "amend_subsidiary_document",
        "change_subsidiary_document_date",
        "change_subsidiary_document_number",
    );

    pub const DELETE: &str = "delete_subsidiary_document";

    pub const LOCK: &str = "lock_subsidiary_document";

    /// Step 6.7: the catalogue's own dedicated print id (1121) — a PDF
    /// download is this rebuild's "print," per `pdf.rs`'s own module doc
    /// comment on `get_voucher_pdf`.
    pub const PRINT: &str = "print_subsidiary_document";

    /// `None` for a transition combination the state machine itself will
    /// reject as invalid — no permission is meaningful for a request that
    /// never had a chance of succeeding.
    pub fn transition(from: &str, to: &str) -> Option<&'static str> {
        match (from, to) {
            ("draft", "confirmed") => Some("approve_subsidiary_document"), // 1116
            ("confirmed", "posted") => Some("post_subsidiary_document_permanently"), // 1117
            ("confirmed", "draft") => Some("revert_subsidiary_document_to_draft"), // 1118
            ("posted", "confirmed") => Some("revert_subsidiary_document_permanent_posting"), // 1145
            _ => None,
        }
    }
}

fn require_permission(auth: &AuthUser, code: &str) -> Result<(), (StatusCode, Json<Value>)> {
    if auth.has_permission(code) {
        Ok(())
    } else {
        forbidden_result("insufficient_permission")
    }
}

fn require_any_permission(
    auth: &AuthUser,
    codes: &[&str],
) -> Result<(), (StatusCode, Json<Value>)> {
    if codes.iter().any(|c| auth.has_permission(c)) {
        Ok(())
    } else {
        forbidden_result("insufficient_permission")
    }
}

fn forbidden_result<T>(error: &str) -> Result<T, (StatusCode, Json<Value>)> {
    Err((StatusCode::FORBIDDEN, Json(json!({ "error": error }))))
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(list_vouchers).post(create_voucher))
        .route(
            "/{id}",
            get(get_voucher).put(update_voucher).delete(delete_voucher),
        )
        .route("/{id}/pdf", get(get_voucher_pdf))
        .route("/{id}/lines", post(add_line))
        .route(
            "/{id}/lines/{line_id}",
            put(update_line).delete(delete_line),
        )
        .route("/{id}/transition", post(transition_voucher))
        .route("/{id}/lock", post(lock_voucher))
        .route("/{id}/unlock", post(unlock_voucher))
}

fn internal_error() -> (StatusCode, Json<Value>) {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(json!({ "error": "internal_error" })),
    )
}
fn not_found(what: &str) -> (StatusCode, Json<Value>) {
    (
        StatusCode::NOT_FOUND,
        Json(json!({ "error": format!("{what}_not_found") })),
    )
}
fn bad_request(error: &str) -> (StatusCode, Json<Value>) {
    (StatusCode::BAD_REQUEST, Json(json!({ "error": error })))
}
fn forbidden(error: &str) -> (StatusCode, Json<Value>) {
    (StatusCode::FORBIDDEN, Json(json!({ "error": error })))
}
fn conflict_or_internal(err: sqlx::Error) -> (StatusCode, Json<Value>) {
    if let sqlx::Error::Database(db_err) = &err {
        if db_err.constraint() == Some("vouchers_number_key") {
            return (
                StatusCode::CONFLICT,
                Json(json!({ "error": "duplicate_voucher_number" })),
            );
        }
    }
    internal_error()
}

#[derive(sqlx::FromRow, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct VoucherRecord {
    id: i64,
    fiscal_year_id: i64,
    voucher_number: i32,
    voucher_date: NaiveDate,
    description: Option<String>,
    total_debit: i64,
    total_credit: i64,
    line_count: i32,
    status: String,
    is_locked: bool,
}

const VOUCHER_COLUMNS: &str = "id, fiscal_year_id, voucher_number, voucher_date, description, \
     total_debit, total_credit, line_count, status::text, is_locked";

#[derive(sqlx::FromRow, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct LineRecord {
    id: i64,
    voucher_id: i64,
    account_id: i64,
    debit_amount: i64,
    credit_amount: i64,
    quantity: bigdecimal::BigDecimal,
    description: Option<String>,
    status: String,
    source_module: i16,
}

fn quantity_from_f64(value: Option<f64>) -> bigdecimal::BigDecimal {
    value
        .and_then(|v| bigdecimal::BigDecimal::try_from(v).ok())
        .unwrap_or_default()
}

const LINE_COLUMNS: &str = "id, voucher_id, account_id, debit_amount, credit_amount, quantity, \
     description, status::text, source_module";

async fn fetch_voucher(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: i64,
    id: i64,
) -> Result<Option<VoucherRecord>, sqlx::Error> {
    sqlx::query_as(&format!(
        "SELECT {VOUCHER_COLUMNS} FROM vouchers WHERE tenant_id = $1 AND id = $2"
    ))
    .bind(tenant_id)
    .bind(id)
    .fetch_optional(&mut **tx)
    .await
}

async fn fetch_line(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: i64,
    voucher_id: i64,
    line_id: i64,
) -> Result<Option<LineRecord>, sqlx::Error> {
    sqlx::query_as(&format!(
        "SELECT {LINE_COLUMNS} FROM voucher_lines \
         WHERE tenant_id = $1 AND voucher_id = $2 AND id = $3"
    ))
    .bind(tenant_id)
    .bind(voucher_id)
    .bind(line_id)
    .fetch_optional(&mut **tx)
    .await
}

/// 03-03-c.md §3.8's `Is_New_Sanad_Valid` — every mutating action requires
/// the voucher's fiscal year to be open. Returns the year's date range too
/// (needed by `create_voucher`'s date-range check, 03-04.md §4.1's "missing
/// validation" the legacy never actually wired up).
async fn fiscal_year_gate(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: i64,
    fiscal_year_id: i64,
) -> Result<Option<(bool, NaiveDate, NaiveDate)>, sqlx::Error> {
    sqlx::query_as(
        "SELECT is_active, start_date, end_date FROM fiscal_years WHERE tenant_id = $1 AND id = $2",
    )
    .bind(tenant_id)
    .bind(fiscal_year_id)
    .fetch_optional(&mut **tx)
    .await
}

/// Not admin-bypassed the way the legacy's `Is_Admin_Or_Valid_Sanad` is
/// (that function fails CLOSED for non-admins when the header row happens
/// to be missing — a bug this schema can't reproduce since the header
/// always exists here) — but the admin-bypass *intent* is preserved:
/// superusers can still edit a locked voucher, everyone else can't.
fn locked_for(voucher: &VoucherRecord, auth: &AuthUser) -> bool {
    voucher.is_locked && !auth.is_superuser
}

// ---- list / get ----------------------------------------------------------

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ListQuery {
    fiscal_year_id: Option<i64>,
    status: Option<String>,
    description: Option<String>,
    sort: Option<String>,
    order: Option<String>,
}

const VOUCHER_SORT_COLUMNS: &[(&str, &str)] = &[
    ("voucherNumber", "voucher_number"),
    ("voucherDate", "voucher_date"),
    ("description", "description"),
    ("totalDebit", "total_debit"),
    ("totalCredit", "total_credit"),
    ("status", "status"),
];

async fn list_vouchers(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(params): Query<ListQuery>,
) -> Result<Json<Vec<VoucherRecord>>, (StatusCode, Json<Value>)> {
    require_any_permission(&auth, permcodes::LIST_VIEW)?;
    let mut tx = db::begin(&state.pool, auth.tenant_id)
        .await
        .map_err(|_| internal_error())?;

    let sql = format!(
        "SELECT {VOUCHER_COLUMNS} FROM vouchers WHERE tenant_id = $1 \
         AND ($2::bigint IS NULL OR fiscal_year_id = $2) \
         AND ($3::text IS NULL OR status = $3) \
         AND ($4::text IS NULL OR description ILIKE '%' || $4 || '%') \
         ORDER BY {}",
        crate::sort::order_by(
            params.sort.as_deref(),
            params.order.as_deref(),
            VOUCHER_SORT_COLUMNS,
            "voucher_date DESC, voucher_number DESC",
        ),
    );

    let rows = sqlx::query_as(&sql)
        .bind(auth.tenant_id)
        .bind(params.fiscal_year_id)
        .bind(&params.status)
        .bind(&params.description)
        .fetch_all(&mut *tx)
        .await
        .map_err(|_| internal_error())?;
    tx.rollback().await.ok();
    Ok(Json(rows))
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct VoucherDetail {
    #[serde(flatten)]
    voucher: VoucherRecord,
    lines: Vec<LineRecord>,
}

async fn get_voucher(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<Json<VoucherDetail>, (StatusCode, Json<Value>)> {
    require_any_permission(&auth, permcodes::LIST_VIEW)?;
    let mut tx = db::begin(&state.pool, auth.tenant_id)
        .await
        .map_err(|_| internal_error())?;
    let Some(voucher) = fetch_voucher(&mut tx, auth.tenant_id, id)
        .await
        .map_err(|_| internal_error())?
    else {
        return Err(not_found("voucher"));
    };
    let lines: Vec<LineRecord> = sqlx::query_as(&format!(
        "SELECT {LINE_COLUMNS} FROM voucher_lines WHERE tenant_id = $1 AND voucher_id = $2 ORDER BY id"
    ))
    .bind(auth.tenant_id)
    .bind(id)
    .fetch_all(&mut *tx)
    .await
    .map_err(|_| internal_error())?;
    tx.rollback().await.ok();
    Ok(Json(VoucherDetail { voucher, lines }))
}

/// Step 6.5: `SanadEditU`/`PrintNu`/`PrintM2U`'s live print path (`PrintMU`
/// is dead, per 04-06-a.md §6.5's own "PrintM is commented out" note, so
/// only `Tanzim` key 1011 -- one signature block -- is ever actually read
/// by a reachable voucher print). Also the print path for every cheque,
/// deposit-slip and petty-cash document (they all carry a real `voucher_id`
/// since 4.2/4.4) -- "print a cheque receipt" is "print its linked
/// voucher," the same endpoint, per `pdf.rs`'s own module doc comment.
async fn get_voucher_pdf(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<axum::response::Response, (StatusCode, Json<Value>)> {
    let mut tx = db::begin(&state.pool, auth.tenant_id)
        .await
        .map_err(|_| internal_error())?;
    let Some(voucher) = fetch_voucher(&mut tx, auth.tenant_id, id)
        .await
        .map_err(|_| internal_error())?
    else {
        return Err(not_found("voucher"));
    };
    require_permission(&auth, permcodes::PRINT)?;
    let lines: Vec<LineRecord> = sqlx::query_as(&format!(
        "SELECT {LINE_COLUMNS} FROM voucher_lines WHERE tenant_id = $1 AND voucher_id = $2 ORDER BY id"
    ))
    .bind(auth.tenant_id)
    .bind(id)
    .fetch_all(&mut *tx)
    .await
    .map_err(|_| internal_error())?;

    let account_ids: Vec<i64> = lines.iter().map(|l| l.account_id).collect();
    let accounts: Vec<(i64, String)> = sqlx::query_as(
        "SELECT id, \
                CASE WHEN subsidiary_code = 0 THEN general_ledger_code::text \
                     WHEN analytic1_code = 0 THEN general_ledger_code::text || '-' || subsidiary_code::text \
                     WHEN analytic2_code = 0 THEN general_ledger_code::text || '-' || subsidiary_code::text || '-' || analytic1_code::text \
                     ELSE general_ledger_code::text || '-' || subsidiary_code::text || '-' || analytic1_code::text || '-' || analytic2_code::text \
                END || ' ' || name \
         FROM accounts WHERE tenant_id = $1 AND id = ANY($2)",
    )
    .bind(auth.tenant_id)
    .bind(&account_ids)
    .fetch_all(&mut *tx)
    .await
    .map_err(|_| internal_error())?;
    let label_for = |account_id: i64| -> String {
        accounts
            .iter()
            .find(|(id, _)| *id == account_id)
            .map(|(_, label)| label.clone())
            .unwrap_or_default()
    };

    let organization_name = crate::pdf::fetch_organization_name(&mut tx, auth.tenant_id)
        .await
        .map_err(|_| internal_error())?;
    let fiscal_year_label: Option<i32> =
        sqlx::query_scalar("SELECT year FROM fiscal_years WHERE tenant_id = $1 AND id = $2")
            .bind(auth.tenant_id)
            .bind(voucher.fiscal_year_id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(|_| internal_error())?;
    let signature_labels =
        crate::pdf::fetch_signature_labels(&mut tx, auth.tenant_id, &["voucher_signature_1"])
            .await
            .map_err(|_| internal_error())?
            .into_iter()
            .filter(|s| !s.is_empty())
            .collect();
    tx.rollback().await.ok();

    let header = crate::pdf::PrintHeader {
        organization_name,
        fiscal_year_caption: fiscal_year_label
            .map(|y| format!("سال مالی {y}"))
            .unwrap_or_default(),
        report_title: "سند حسابداری".to_string(),
        period_caption: None,
        amount_in_words: None,
        signature_labels,
    };
    let pdf_lines: Vec<crate::pdf::VoucherLineData> = lines
        .iter()
        .map(|l| crate::pdf::VoucherLineData {
            account_label: label_for(l.account_id),
            description: l.description.clone().unwrap_or_default(),
            debit: l.debit_amount,
            credit: l.credit_amount,
        })
        .collect();

    let pdf = crate::pdf::render_voucher_pdf(
        header,
        voucher.voucher_number,
        &voucher.voucher_date.format("%Y/%m/%d").to_string(),
        voucher.description.as_deref().unwrap_or(""),
        &pdf_lines,
        voucher.total_debit,
        voucher.total_credit,
    )
    .map_err(|_| internal_error())?;

    Ok((
        [
            (
                axum::http::header::CONTENT_TYPE,
                "application/pdf".to_string(),
            ),
            (
                axum::http::header::CONTENT_DISPOSITION,
                format!("inline; filename=\"voucher-{id}.pdf\""),
            ),
        ],
        pdf,
    )
        .into_response())
}

// ---- create / update header ----------------------------------------------

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateVoucherRequest {
    fiscal_year_id: i64,
    voucher_number: Option<i32>,
    voucher_date: NaiveDate,
    description: String,
}

/// 03-04.md §4.1 header checks #1-#5 (voucher-number handling is delegated
/// to atomic allocation or the DB's unique constraint rather than a
/// duplicate pre-check — check #1 doesn't apply the way it does in the
/// legacy since a blank number just means "auto-allocate" here), plus the
/// NEW fiscal-year-range check the legacy never actually enforced.
async fn create_voucher(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<CreateVoucherRequest>,
) -> Result<(StatusCode, Json<Value>), (StatusCode, Json<Value>)> {
    require_permission(&auth, permcodes::POST_LEDGER)?;
    if req.description.trim().is_empty() {
        return Err(bad_request("description_required")); // check #5
    }

    let mut tx = db::begin(&state.pool, auth.tenant_id)
        .await
        .map_err(|_| internal_error())?;

    let Some((is_active, start_date, end_date)) =
        fiscal_year_gate(&mut tx, auth.tenant_id, req.fiscal_year_id)
            .await
            .map_err(|_| internal_error())?
    else {
        return Err(not_found("fiscal_year"));
    };
    if !is_active {
        return Err(forbidden("fiscal_year_closed"));
    }
    if req.voucher_date < start_date || req.voucher_date > end_date {
        return Err(bad_request("date_outside_fiscal_year")); // check #4 + the new range check
    }

    let voucher_number = match req.voucher_number {
        Some(n) if n > 0 => n,
        Some(_) => return Err(bad_request("invalid_voucher_number")),
        None => sqlx::query_scalar(
            "UPDATE fiscal_years SET next_voucher_number = next_voucher_number + 1 \
             WHERE id = $1 RETURNING next_voucher_number - 1",
        )
        .bind(req.fiscal_year_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(|_| internal_error())?,
    };

    let id: i64 = sqlx::query_scalar(
        "INSERT INTO vouchers \
         (tenant_id, fiscal_year_id, voucher_number, voucher_date, description, created_by) \
         VALUES ($1, $2, $3, $4, $5, $6) RETURNING id",
    )
    .bind(auth.tenant_id)
    .bind(req.fiscal_year_id)
    .bind(voucher_number)
    .bind(req.voucher_date)
    .bind(req.description.trim())
    .bind(auth.user_id)
    .fetch_one(&mut *tx)
    .await
    .map_err(conflict_or_internal)?;

    audit::record_mutation(
        &mut tx,
        auth.tenant_id,
        "vouchers",
        id,
        "insert",
        Some(auth.user_id),
        None,
        Some(json!({ "voucherNumber": voucher_number, "voucherDate": req.voucher_date })),
    )
    .await
    .map_err(|_| internal_error())?;

    tx.commit().await.map_err(|_| internal_error())?;
    Ok((StatusCode::CREATED, Json(json!({ "id": id }))))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct UpdateVoucherRequest {
    voucher_date: NaiveDate,
    description: String,
    /// Optional — matches `SanadViewU`'s change-number action. Omit to leave the number as-is.
    voucher_number: Option<i32>,
}

/// "Edit voucher header/lines requires DM_TX = 0 and M_TX = 0" (03-03-b.md
/// §3.6) — draft only, and locked vouchers are off-limits to non-superusers.
async fn update_voucher(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
    Json(req): Json<UpdateVoucherRequest>,
) -> Result<StatusCode, (StatusCode, Json<Value>)> {
    if req.description.trim().is_empty() {
        return Err(bad_request("description_required"));
    }
    let mut tx = db::begin(&state.pool, auth.tenant_id)
        .await
        .map_err(|_| internal_error())?;
    let Some(voucher) = fetch_voucher(&mut tx, auth.tenant_id, id)
        .await
        .map_err(|_| internal_error())?
    else {
        return Err(not_found("voucher"));
    };
    if voucher.status != "draft" {
        return Err(bad_request("not_draft"));
    }
    if locked_for(&voucher, &auth) {
        return Err(forbidden("voucher_locked"));
    }
    // 03-13-permissions.md §13.2's per-field granularity: the base "amend" id is always
    // required, plus the specific date-change / number-change id for whichever fields
    // actually changed.
    let (base_perm, date_perm, number_perm) = permcodes::HEADER_EDIT;
    require_permission(&auth, base_perm)?;
    if req.voucher_date != voucher.voucher_date {
        require_permission(&auth, date_perm)?;
    }
    if let Some(n) = req.voucher_number {
        if n != voucher.voucher_number {
            require_permission(&auth, number_perm)?;
        }
    }
    let Some((is_active, start_date, end_date)) =
        fiscal_year_gate(&mut tx, auth.tenant_id, voucher.fiscal_year_id)
            .await
            .map_err(|_| internal_error())?
    else {
        return Err(internal_error());
    };
    if !is_active {
        return Err(forbidden("fiscal_year_closed"));
    }
    if req.voucher_date < start_date || req.voucher_date > end_date {
        return Err(bad_request("date_outside_fiscal_year"));
    }

    let new_number = req.voucher_number.unwrap_or(voucher.voucher_number);
    if new_number <= 0 {
        return Err(bad_request("invalid_voucher_number"));
    }

    sqlx::query(
        "UPDATE vouchers SET voucher_date = $1, description = $2, voucher_number = $3, \
         updated_at = now(), updated_by = $4 WHERE id = $5",
    )
    .bind(req.voucher_date)
    .bind(req.description.trim())
    .bind(new_number)
    .bind(auth.user_id)
    .bind(id)
    .execute(&mut *tx)
    .await
    .map_err(conflict_or_internal)?;
    // Every line carries a copy of the header date (03-03-a.md §3.3) — keep it in sync.
    sqlx::query(
        "UPDATE voucher_lines SET line_date = $1, updated_at = now() WHERE voucher_id = $2",
    )
    .bind(req.voucher_date)
    .bind(id)
    .execute(&mut *tx)
    .await
    .map_err(|_| internal_error())?;

    audit::record_mutation(
        &mut tx,
        auth.tenant_id,
        "vouchers",
        id,
        "update",
        Some(auth.user_id),
        Some(json!({ "voucherDate": voucher.voucher_date, "description": voucher.description, "voucherNumber": voucher.voucher_number })),
        Some(json!({ "voucherDate": req.voucher_date, "description": req.description.trim(), "voucherNumber": new_number })),
    )
    .await
    .map_err(|_| internal_error())?;

    tx.commit().await.map_err(|_| internal_error())?;
    Ok(StatusCode::NO_CONTENT)
}

// ---- lines -----------------------------------------------------------

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct LineRequest {
    account_id: i64,
    debit: i64,
    credit: i64,
    description: String,
    #[serde(default)]
    quantity: Option<f64>,
}

/// 03-04.md §4.2's line checks, in order: account resolves to a leaf (#1),
/// exactly one side nonzero and positive (#2 — structural, checked here
/// rather than left to the DB's CHECK constraints so the error is
/// friendly), description required (#3).
async fn validate_line_request(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: i64,
    req: &LineRequest,
) -> Result<(), (StatusCode, Json<Value>)> {
    if req.debit < 0 || req.credit < 0 {
        return Err(bad_request("negative_amount"));
    }
    if (req.debit > 0) == (req.credit > 0) {
        // both zero, or both nonzero — either way invalid (#2)
        return Err(bad_request(if req.debit == 0 {
            "amount_required"
        } else {
            "both_sides_filled"
        }));
    }
    if req.description.trim().is_empty() {
        return Err(bad_request("description_required")); // #3
    }
    let child_count: Option<i32> =
        sqlx::query_scalar("SELECT child_count FROM accounts WHERE tenant_id = $1 AND id = $2")
            .bind(tenant_id)
            .bind(req.account_id)
            .fetch_optional(&mut **tx)
            .await
            .map_err(|_| internal_error())?;
    match child_count {
        None => return Err(bad_request("account_not_found")),
        Some(c) if c > 0 => return Err(bad_request("account_not_leaf")), // #1
        _ => {}
    }
    Ok(())
}

async fn add_line(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
    Json(req): Json<LineRequest>,
) -> Result<(StatusCode, Json<Value>), (StatusCode, Json<Value>)> {
    let mut tx = db::begin(&state.pool, auth.tenant_id)
        .await
        .map_err(|_| internal_error())?;
    let Some(voucher) = fetch_voucher(&mut tx, auth.tenant_id, id)
        .await
        .map_err(|_| internal_error())?
    else {
        return Err(not_found("voucher"));
    };
    if voucher.status != "draft" {
        return Err(bad_request("not_draft"));
    }
    if locked_for(&voucher, &auth) {
        return Err(forbidden("voucher_locked"));
    }
    require_permission(&auth, permcodes::AMEND_LINE)?;
    validate_line_request(&mut tx, auth.tenant_id, &req).await?;

    let line_id: i64 = sqlx::query_scalar(
        "INSERT INTO voucher_lines \
         (tenant_id, voucher_id, fiscal_year_id, line_date, debit_amount, credit_amount, quantity, \
          description, account_id, status, created_by) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10::voucher_status, $11) RETURNING id",
    )
    .bind(auth.tenant_id)
    .bind(id)
    .bind(voucher.fiscal_year_id)
    .bind(voucher.voucher_date)
    .bind(req.debit)
    .bind(req.credit)
    .bind(quantity_from_f64(req.quantity))
    .bind(req.description.trim())
    .bind(req.account_id)
    .bind(&voucher.status)
    .bind(auth.user_id)
    .fetch_one(&mut *tx)
    .await
    .map_err(|_| internal_error())?;

    sqlx::query(
        "UPDATE vouchers SET total_debit = total_debit + $1, total_credit = total_credit + $2, \
         line_count = line_count + 1, updated_at = now(), updated_by = $3 WHERE id = $4",
    )
    .bind(req.debit)
    .bind(req.credit)
    .bind(auth.user_id)
    .bind(id)
    .execute(&mut *tx)
    .await
    .map_err(|_| internal_error())?;

    audit::record_mutation(
        &mut tx,
        auth.tenant_id,
        "voucher_lines",
        line_id,
        "insert",
        Some(auth.user_id),
        None,
        Some(json!({ "voucherId": id, "accountId": req.account_id, "debit": req.debit, "credit": req.credit })),
    )
    .await
    .map_err(|_| internal_error())?;

    tx.commit().await.map_err(|_| internal_error())?;
    Ok((StatusCode::CREATED, Json(json!({ "id": line_id }))))
}

/// Guards both draft-only editing AND 03-03-a.md §3.4's immutability table
/// ("Edit line in voucher editor" / `M_Id > 0` -> rejected) — a generated
/// line can never be touched from this endpoint, full stop, regardless of
/// voucher state.
async fn update_line(
    State(state): State<AppState>,
    auth: AuthUser,
    Path((id, line_id)): Path<(i64, i64)>,
    Json(req): Json<LineRequest>,
) -> Result<StatusCode, (StatusCode, Json<Value>)> {
    let mut tx = db::begin(&state.pool, auth.tenant_id)
        .await
        .map_err(|_| internal_error())?;
    let Some(voucher) = fetch_voucher(&mut tx, auth.tenant_id, id)
        .await
        .map_err(|_| internal_error())?
    else {
        return Err(not_found("voucher"));
    };
    if voucher.status != "draft" {
        return Err(bad_request("not_draft"));
    }
    if locked_for(&voucher, &auth) {
        return Err(forbidden("voucher_locked"));
    }
    require_permission(&auth, permcodes::AMEND_LINE)?;
    let Some(line) = fetch_line(&mut tx, auth.tenant_id, id, line_id)
        .await
        .map_err(|_| internal_error())?
    else {
        return Err(not_found("line"));
    };
    if line.source_module != 0 {
        return Err(forbidden("generated_line_immutable"));
    }
    validate_line_request(&mut tx, auth.tenant_id, &req).await?;

    sqlx::query(
        "UPDATE voucher_lines SET account_id = $1, debit_amount = $2, credit_amount = $3, \
         quantity = $4, description = $5, updated_at = now(), updated_by = $6 WHERE id = $7",
    )
    .bind(req.account_id)
    .bind(req.debit)
    .bind(req.credit)
    .bind(quantity_from_f64(req.quantity))
    .bind(req.description.trim())
    .bind(auth.user_id)
    .bind(line_id)
    .execute(&mut *tx)
    .await
    .map_err(|_| internal_error())?;

    sqlx::query(
        "UPDATE vouchers SET total_debit = total_debit - $1 + $2, total_credit = total_credit - $3 + $4, \
         updated_at = now(), updated_by = $5 WHERE id = $6",
    )
    .bind(line.debit_amount)
    .bind(req.debit)
    .bind(line.credit_amount)
    .bind(req.credit)
    .bind(auth.user_id)
    .bind(id)
    .execute(&mut *tx)
    .await
    .map_err(|_| internal_error())?;

    audit::record_mutation(
        &mut tx,
        auth.tenant_id,
        "voucher_lines",
        line_id,
        "update",
        Some(auth.user_id),
        Some(json!({ "debit": line.debit_amount, "credit": line.credit_amount })),
        Some(json!({ "debit": req.debit, "credit": req.credit })),
    )
    .await
    .map_err(|_| internal_error())?;

    tx.commit().await.map_err(|_| internal_error())?;
    Ok(StatusCode::NO_CONTENT)
}

async fn delete_line(
    State(state): State<AppState>,
    auth: AuthUser,
    Path((id, line_id)): Path<(i64, i64)>,
) -> Result<StatusCode, (StatusCode, Json<Value>)> {
    let mut tx = db::begin(&state.pool, auth.tenant_id)
        .await
        .map_err(|_| internal_error())?;
    let Some(voucher) = fetch_voucher(&mut tx, auth.tenant_id, id)
        .await
        .map_err(|_| internal_error())?
    else {
        return Err(not_found("voucher"));
    };
    if voucher.status != "draft" {
        return Err(bad_request("not_draft"));
    }
    if locked_for(&voucher, &auth) {
        return Err(forbidden("voucher_locked"));
    }
    require_permission(&auth, permcodes::AMEND_LINE)?;
    let Some(line) = fetch_line(&mut tx, auth.tenant_id, id, line_id)
        .await
        .map_err(|_| internal_error())?
    else {
        return Err(not_found("line"));
    };
    if line.source_module != 0 {
        return Err(forbidden("generated_line_immutable"));
    }

    sqlx::query("DELETE FROM voucher_lines WHERE id = $1")
        .bind(line_id)
        .execute(&mut *tx)
        .await
        .map_err(|_| internal_error())?;

    sqlx::query(
        "UPDATE vouchers SET total_debit = total_debit - $1, total_credit = total_credit - $2, \
         line_count = line_count - 1, updated_at = now(), updated_by = $3 WHERE id = $4",
    )
    .bind(line.debit_amount)
    .bind(line.credit_amount)
    .bind(auth.user_id)
    .bind(id)
    .execute(&mut *tx)
    .await
    .map_err(|_| internal_error())?;

    audit::record_mutation(
        &mut tx,
        auth.tenant_id,
        "voucher_lines",
        line_id,
        "delete",
        Some(auth.user_id),
        Some(json!({ "accountId": line.account_id, "debit": line.debit_amount, "credit": line.credit_amount })),
        None,
    )
    .await
    .map_err(|_| internal_error())?;

    tx.commit().await.map_err(|_| internal_error())?;
    Ok(StatusCode::NO_CONTENT)
}

// ---- state machine -----------------------------------------------------

#[derive(Deserialize)]
struct TransitionRequest {
    to: String,
}

/// 03-03-b.md §3.6's four transitions. Balance (`total_debit = total_credit`,
/// integers — never the legacy's formatted-string footer comparison,
/// 03-04.md §4.1 check #7) and "at least one line" are checked ONLY on
/// draft -> confirmed, exactly like the legacy's sole balance check
/// (SanadViewU.pas:298,301) — a draft may be freely unbalanced.
async fn transition_voucher(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
    Json(req): Json<TransitionRequest>,
) -> Result<StatusCode, (StatusCode, Json<Value>)> {
    let mut tx = db::begin(&state.pool, auth.tenant_id)
        .await
        .map_err(|_| internal_error())?;
    let Some(voucher) = fetch_voucher(&mut tx, auth.tenant_id, id)
        .await
        .map_err(|_| internal_error())?
    else {
        return Err(not_found("voucher"));
    };
    if locked_for(&voucher, &auth) {
        return Err(forbidden("voucher_locked"));
    }
    if let Some(perm) = permcodes::transition(&voucher.status, &req.to) {
        require_permission(&auth, perm)?;
    } // else: not a recognised transition at all — let the match below 400 it, no permission to check
    let Some((is_active, _, _)) = fiscal_year_gate(&mut tx, auth.tenant_id, voucher.fiscal_year_id)
        .await
        .map_err(|_| internal_error())?
    else {
        return Err(internal_error());
    };
    if !is_active {
        return Err(forbidden("fiscal_year_closed"));
    }

    let valid = match (voucher.status.as_str(), req.to.as_str()) {
        ("draft", "confirmed") => {
            if voucher.line_count == 0 {
                return Err(bad_request("voucher_empty")); // header check #6
            }
            if voucher.total_debit != voucher.total_credit {
                return Err(bad_request("voucher_not_balanced")); // header check #7
            }
            true
        }
        ("confirmed", "draft") | ("confirmed", "posted") | ("posted", "confirmed") => true,
        _ => false,
    };
    if !valid {
        return Err(bad_request("invalid_transition"));
    }

    sqlx::query("UPDATE vouchers SET status = $1::voucher_status, updated_at = now(), updated_by = $2 WHERE id = $3")
        .bind(&req.to)
        .bind(auth.user_id)
        .bind(id)
        .execute(&mut *tx)
        .await
        .map_err(|_| internal_error())?;
    sqlx::query("UPDATE voucher_lines SET status = $1::voucher_status, updated_at = now() WHERE voucher_id = $2")
        .bind(&req.to)
        .bind(id)
        .execute(&mut *tx)
        .await
        .map_err(|_| internal_error())?;

    audit::record_mutation(
        &mut tx,
        auth.tenant_id,
        "vouchers",
        id,
        "update",
        Some(auth.user_id),
        Some(json!({ "status": voucher.status })),
        Some(json!({ "status": req.to })),
    )
    .await
    .map_err(|_| internal_error())?;

    tx.commit().await.map_err(|_| internal_error())?;
    Ok(StatusCode::NO_CONTENT)
}

// ---- lock / unlock (independent of the state machine, 03-03-b.md §3.6) --

async fn set_locked(
    state: &AppState,
    auth: &AuthUser,
    id: i64,
    locked: bool,
) -> Result<StatusCode, (StatusCode, Json<Value>)> {
    let mut tx = db::begin(&state.pool, auth.tenant_id)
        .await
        .map_err(|_| internal_error())?;
    let Some(voucher) = fetch_voucher(&mut tx, auth.tenant_id, id)
        .await
        .map_err(|_| internal_error())?
    else {
        return Err(not_found("voucher"));
    };
    require_permission(auth, permcodes::LOCK)?;
    sqlx::query(
        "UPDATE vouchers SET is_locked = $1, updated_at = now(), updated_by = $2 WHERE id = $3",
    )
    .bind(locked)
    .bind(auth.user_id)
    .bind(id)
    .execute(&mut *tx)
    .await
    .map_err(|_| internal_error())?;
    audit::record_mutation(
        &mut tx,
        auth.tenant_id,
        "vouchers",
        id,
        "update",
        Some(auth.user_id),
        Some(json!({ "isLocked": voucher.is_locked })),
        Some(json!({ "isLocked": locked })),
    )
    .await
    .map_err(|_| internal_error())?;
    tx.commit().await.map_err(|_| internal_error())?;
    Ok(StatusCode::NO_CONTENT)
}

async fn lock_voucher(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<StatusCode, (StatusCode, Json<Value>)> {
    set_locked(&state, &auth, id, true).await
}

async fn unlock_voucher(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<StatusCode, (StatusCode, Json<Value>)> {
    set_locked(&state, &auth, id, false).await
}

// ---- delete --------------------------------------------------------------

/// 03-04.md §4.5's guards, in order: fiscal year open (checked above via
/// the same gate every mutation uses), draft only (`Max(M_Tx) = 0`), no
/// generated lines (`Max(M_Id) > 0` -> rejected — "use the auxiliary
/// programs"). `ON DELETE CASCADE` removes the lines.
async fn delete_voucher(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<StatusCode, (StatusCode, Json<Value>)> {
    let mut tx = db::begin(&state.pool, auth.tenant_id)
        .await
        .map_err(|_| internal_error())?;
    let Some(voucher) = fetch_voucher(&mut tx, auth.tenant_id, id)
        .await
        .map_err(|_| internal_error())?
    else {
        return Err(not_found("voucher"));
    };
    if voucher.status != "draft" {
        return Err(bad_request("not_draft"));
    }
    if locked_for(&voucher, &auth) {
        return Err(forbidden("voucher_locked"));
    }
    require_permission(&auth, permcodes::DELETE)?;
    let has_generated_lines: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM voucher_lines WHERE voucher_id = $1 AND source_module <> 0)",
    )
    .bind(id)
    .fetch_one(&mut *tx)
    .await
    .map_err(|_| internal_error())?;
    if has_generated_lines {
        return Err(forbidden("has_generated_lines"));
    }

    sqlx::query("DELETE FROM vouchers WHERE id = $1")
        .bind(id)
        .execute(&mut *tx)
        .await
        .map_err(|_| internal_error())?;

    audit::record_mutation(
        &mut tx,
        auth.tenant_id,
        "vouchers",
        id,
        "delete",
        Some(auth.user_id),
        Some(json!({ "voucherNumber": voucher.voucher_number, "lineCount": voucher.line_count })),
        None,
    )
    .await
    .map_err(|_| internal_error())?;

    tx.commit().await.map_err(|_| internal_error())?;
    Ok(StatusCode::NO_CONTENT)
}
