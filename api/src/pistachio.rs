//! Step 5.6 (docs/phase-5-inventory.md §5.6): the pistachio deduction calculator — the domain's
//! signature formula, finally reachable in the UI (B19). specs/05-inventory/05-08-a.md §8.2 is
//! the exact arithmetic to preserve; §8.0.1 documents why the legacy version never ran at all
//! (hidden panel, no `OnClick` on the Save button, nothing ever writes the record type) — P1's
//! math is correct, P2 (the reachable pistachio path) just never exposed it. This module is that
//! exposure: a real, first-class step in the purchase-invoice flow, not a hidden panel.
//!
//! **Preserved exactly, per the Build bullet's own "preserve exactly, not fix the defect" framing**:
//! moisture and blanks are computed independently off the same `gross_weight` base and added, not
//! compounded; `other_deductions_kg` is entered directly in kilograms, never a percentage; only
//! `net_weight` is floored at zero — `total_deductions` itself may legitimately exceed
//! `gross_weight` and is stored and displayed as-is (§8.2.2's own explicit list of what must not
//! change).
//!
//! **`line_amount = round(net_weight × unit_price)`** reuses this project's one rounding decision
//! (`money::round_to_rial`, `RoundingMode::HalfUp`) rather than Delphi's banker's rounding — the
//! Build bullet's own recommendation, chosen and documented once in `money.rs` rather than
//! re-decided here. §8.2.3 Example A (the worked example this step's own manual test reproduces)
//! divides to a whole number so the two modes agree there; §8.2.3 Example C is the one worked case
//! where they genuinely diverge, reproduced as a regression test in `money.rs`.
//!
//! **Reachability, the actual B19 fix**: `create_pistachio_line` calls the *identical*
//! `inventory_documents::insert_line_and_bump_totals` helper `add_line` (5.2) uses — a pistachio
//! purchase line is a real `inventory_document_lines` row (`quantity = net_weight`), not a
//! parallel, second document type. `calculate_deduction_preview` is the stateless "no hidden panel,
//! no dead Save button" fix for live recomputation: the UI calls it on every field change (this
//! step's manual test #2's "confirm the UI does let you see this before saving") without
//! persisting anything, exactly mirroring `BascVChange`'s "recomputes on every keystroke" intent
//! but as an explicit, discoverable request-response action rather than a Delphi `OnChange` handler.
//!
//! **Judgment call**: the mandatory-field validation (bale count, gross weight, unit price > 0) is
//! real here — §8.2.2's own finding that the legacy's red labels were "cosmetic only" and
//! `BSaveClick` "has no validation at all" is the exact defect this step is required to fix (this
//! step's own manual test #3), not preserve.

use crate::{
    audit,
    auth::AuthUser,
    db,
    inventory_documents::{self, LineAmounts},
    money, AppState,
};
use axum::{extract::State, http::StatusCode, routing::post, Json, Router};
use bigdecimal::{BigDecimal, Zero};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

pub fn router() -> Router<AppState> {
    Router::new().route("/calculate", post(calculate_preview))
}

fn internal_error() -> (StatusCode, Json<Value>) {
    (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": "internal_error" })))
}
fn bad_request(error: &str) -> (StatusCode, Json<Value>) {
    (StatusCode::BAD_REQUEST, Json(json!({ "error": error })))
}
fn not_found(error: &str) -> (StatusCode, Json<Value>) {
    (StatusCode::NOT_FOUND, Json(json!({ "error": error })))
}

/// The one valid set of `AdlV` combo values (§8.2.1: `100 گرم` / `200 گرم` / `یک کیلو`) — a bare
/// `TEditDecimal` in the legacy accepted anything; this rebuild enforces the closed set the
/// combo box actually offered, since a `tare_allowance_kg` of e.g. `0.15` has no meaning.
const VALID_TARE_ALLOWANCES: [&str; 3] = ["0.1", "0.2", "1.0"];

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeductionInput {
    pub bale_count: i32,
    pub tare_allowance_kg: BigDecimal,
    pub gross_weight_kg: BigDecimal,
    #[serde(default)]
    pub moisture_pct: BigDecimal,
    #[serde(default)]
    pub blank_pct: BigDecimal,
    #[serde(default)]
    pub other_deductions_kg: BigDecimal,
    pub unit_price: i64,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DeductionResult {
    pub tare_deduction_kg: BigDecimal,
    pub moisture_deduction_kg: BigDecimal,
    pub blank_deduction_kg: BigDecimal,
    pub total_deduction_kg: BigDecimal,
    pub net_weight_kg: BigDecimal,
    pub line_amount: i64,
}

/// This step's own manual test #3: bale count, gross weight and unit price are real required
/// fields — not the legacy's cosmetic red-label-only validation that let a zero-value save
/// through unchecked (§8.2.2's own finding).
fn validate_mandatory_fields(input: &DeductionInput) -> Result<(), (StatusCode, Json<Value>)> {
    if input.bale_count <= 0 {
        return Err(bad_request("bale_count_required"));
    }
    if input.gross_weight_kg <= BigDecimal::zero() {
        return Err(bad_request("gross_weight_required"));
    }
    if input.unit_price <= 0 {
        return Err(bad_request("unit_price_required"));
    }
    if !VALID_TARE_ALLOWANCES.iter().any(|v| v.parse::<BigDecimal>().unwrap() == input.tare_allowance_kg) {
        return Err(bad_request("invalid_tare_allowance")); // §8.2.1: AdlV's three combo values only
    }
    if input.moisture_pct < BigDecimal::zero() || input.blank_pct < BigDecimal::zero() {
        return Err(bad_request("invalid_percentage"));
    }
    if input.other_deductions_kg < BigDecimal::zero() {
        return Err(bad_request("invalid_other_deductions"));
    }
    Ok(())
}

/// §8.2.2's formulas, preserved exactly: percentages apply independently to `gross_weight` (not
/// compounded), `other_deductions_kg` is added untransformed, and only `net_weight` is floored at
/// zero — `total_deduction` itself is left as-is even when it exceeds `gross_weight` (§8.2.3
/// Example B, the deduction floor).
/// Weight figures are normalised to 3 decimal places (`bigdecimal`'s intermediate division
/// otherwise carries an unpredictable scale, e.g. `1877.00` rather than `1877.0`) — matching this
/// column family's declared precision (`numeric(10,3)`/`(14,3)`) everywhere else in the schema,
/// not a new decision made here.
fn scale3(value: bigdecimal::BigDecimal) -> bigdecimal::BigDecimal {
    value.with_scale_round(3, bigdecimal::RoundingMode::HalfUp)
}

/// `pub` (not module-private) since Step 7.5's reconciliation harness (`api/tests/reconciliation.rs`)
/// calls this directly — the exact formula-under-test, not a re-exercise of the HTTP/auth stack
/// already covered by this module's own tests below.
pub fn compute_deduction(input: &DeductionInput) -> DeductionResult {
    let tare_deduction_kg = scale3(BigDecimal::from(input.bale_count) * &input.tare_allowance_kg);
    let moisture_deduction_kg = scale3(&input.moisture_pct * &input.gross_weight_kg / BigDecimal::from(100));
    let blank_deduction_kg = scale3(&input.blank_pct * &input.gross_weight_kg / BigDecimal::from(100));
    let total_deduction_kg =
        scale3(&tare_deduction_kg + &moisture_deduction_kg + &blank_deduction_kg + &input.other_deductions_kg);

    let net_weight_kg = if input.gross_weight_kg < total_deduction_kg {
        BigDecimal::zero() // §8.2.2: only net_weight floors, the deduction total itself does not
    } else {
        scale3(&input.gross_weight_kg - &total_deduction_kg)
    };

    let line_amount = money::round_to_rial(&(&net_weight_kg * BigDecimal::from(input.unit_price)));

    DeductionResult {
        tare_deduction_kg,
        moisture_deduction_kg,
        blank_deduction_kg,
        total_deduction_kg,
        net_weight_kg,
        line_amount,
    }
}

/// The stateless preview — no persistence, no document required. The UI's "reachable, discoverable
/// button" replacement for `BascVChange`'s per-keystroke recompute (§8.2's own closing note).
async fn calculate_preview(
    _auth: AuthUser,
    Json(input): Json<DeductionInput>,
) -> Result<Json<DeductionResult>, (StatusCode, Json<Value>)> {
    validate_mandatory_fields(&input)?;
    Ok(Json(compute_deduction(&input)))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreatePistachioLineRequest {
    pub item_id: i64,
    #[serde(flatten)]
    pub deduction: DeductionInput,
    #[serde(default)]
    pub description: Option<String>,
}

/// The real create-line action — a pistachio purchase line is an ordinary `inventory_document_
/// lines` row (`quantity = net_weight`) plus one linked `pistachio_deduction_details` row, both in
/// the same transaction as every other line mutation in this system. Discount and VAT are hard
/// zero (§7.5: "Pistachio receipt: always 0" for both), matching the legacy exactly — pistachio
/// pricing is negotiated per lot, not discounted or taxed like an ordinary invoice line.
pub(crate) async fn create_pistachio_line(
    State(state): State<AppState>,
    auth: AuthUser,
    axum::extract::Path(document_id): axum::extract::Path<i64>,
    Json(req): Json<CreatePistachioLineRequest>,
) -> Result<(StatusCode, Json<Value>), (StatusCode, Json<Value>)> {
    inventory_documents::require_permission(&auth, inventory_documents::permcodes::AMEND)?;
    validate_mandatory_fields(&req.deduction)?;

    let mut tx = db::begin(&state.pool, auth.tenant_id).await.map_err(|_| internal_error())?;

    let Some(document) = inventory_documents::fetch_document(&mut tx, auth.tenant_id, document_id)
        .await
        .map_err(|_| internal_error())?
    else {
        return Err(not_found("document_not_found"));
    };
    if document.status != "draft" {
        return Err(bad_request("not_draft"));
    }
    if document.document_type != "receipt" {
        return Err(bad_request("pistachio_purchase_only")); // §8.3.4/§3.3: a purchase pipeline only
    }

    let grade_id: Option<i64> =
        sqlx::query_scalar("SELECT pistachio_grade_id FROM items WHERE tenant_id = $1 AND id = $2")
            .bind(auth.tenant_id)
            .bind(req.item_id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(|_| internal_error())?
            .flatten();
    if grade_id.is_none() {
        return Err(bad_request("item_not_pistachio_grade")); // §8.1.1: explicit FK, no shared-integer guessing
    }

    let result = compute_deduction(&req.deduction);
    let amounts = LineAmounts { gross: result.line_amount, discount: 0, tax: 0, total: result.line_amount };

    let line_id = inventory_documents::insert_line_and_bump_totals(
        &mut tx,
        auth.tenant_id,
        &document,
        req.item_id,
        &result.net_weight_kg,
        req.deduction.unit_price,
        &amounts,
        req.description.as_deref().map(str::trim),
        auth.user_id,
    )
    .await
    .map_err(|_| internal_error())?;

    sqlx::query(
        "INSERT INTO pistachio_deduction_details \
         (tenant_id, document_line_id, bale_count, tare_allowance_kg, gross_weight_kg, moisture_pct, \
          blank_pct, other_deductions_kg, tare_deduction_kg, moisture_deduction_kg, blank_deduction_kg, \
          total_deduction_kg, net_weight_kg, created_by) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)",
    )
    .bind(auth.tenant_id)
    .bind(line_id)
    .bind(req.deduction.bale_count)
    .bind(&req.deduction.tare_allowance_kg)
    .bind(&req.deduction.gross_weight_kg)
    .bind(&req.deduction.moisture_pct)
    .bind(&req.deduction.blank_pct)
    .bind(&req.deduction.other_deductions_kg)
    .bind(&result.tare_deduction_kg)
    .bind(&result.moisture_deduction_kg)
    .bind(&result.blank_deduction_kg)
    .bind(&result.total_deduction_kg)
    .bind(&result.net_weight_kg)
    .bind(auth.user_id)
    .execute(&mut *tx)
    .await
    .map_err(|_| internal_error())?;

    audit::record_mutation(
        &mut tx, auth.tenant_id, "inventory_document_lines", line_id, "insert", Some(auth.user_id), None,
        Some(json!({
            "documentId": document_id, "itemId": req.item_id, "pistachio": true,
            "netWeightKg": result.net_weight_kg.to_string(), "lineAmount": result.line_amount,
        })),
    )
    .await
    .map_err(|_| internal_error())?;

    tx.commit().await.map_err(|_| internal_error())?;
    Ok((StatusCode::CREATED, Json(json!({ "id": line_id, "netWeightKg": result.net_weight_kg.to_string(), "lineAmount": result.line_amount }))))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input(bale_count: i32, tare: &str, gross: &str, moisture: &str, blank: &str, other: &str, price: i64) -> DeductionInput {
        DeductionInput {
            bale_count,
            tare_allowance_kg: tare.parse().unwrap(),
            gross_weight_kg: gross.parse().unwrap(),
            moisture_pct: moisture.parse().unwrap(),
            blank_pct: blank.parse().unwrap(),
            other_deductions_kg: other.parse().unwrap(),
            unit_price: price,
        }
    }

    /// §8.2.3 Example A, verbatim.
    #[test]
    fn example_a_ordinary_lot() {
        let result = compute_deduction(&input(40, "0.2", "2000.0", "3.5", "2.0", "5.0", 1_250_000));
        assert_eq!(result.tare_deduction_kg, "8.000".parse::<BigDecimal>().unwrap());
        assert_eq!(result.moisture_deduction_kg, "70.000".parse::<BigDecimal>().unwrap());
        assert_eq!(result.blank_deduction_kg, "40.000".parse::<BigDecimal>().unwrap());
        assert_eq!(result.total_deduction_kg, "123.000".parse::<BigDecimal>().unwrap());
        assert_eq!(result.net_weight_kg, "1877.000".parse::<BigDecimal>().unwrap());
        assert_eq!(result.line_amount, 2_346_250_000);
    }

    /// §8.2.3 Example B — the deduction floor: total deductions (565 kg) exceed gross (500 kg),
    /// net weight floors to 0, but the deduction total itself is left as-is (not floored).
    #[test]
    fn example_b_deduction_floor() {
        let result = compute_deduction(&input(40, "1.0", "500.0", "60", "45", "0", 1_000_000));
        assert_eq!(result.total_deduction_kg, "565.000".parse::<BigDecimal>().unwrap()); // exceeds gross, shown as-is
        assert_eq!(result.net_weight_kg, BigDecimal::zero());
        assert_eq!(result.line_amount, 0);
    }

    #[test]
    fn percentages_are_independent_not_compounded() {
        // 3.5% + 2% of 2000 = 70 + 40 = 110, NOT 2000*(1-0.035)*(1-0.02) which would give 109.3-ish.
        let result = compute_deduction(&input(0, "0.1", "2000.0", "3.5", "2.0", "0", 1));
        let combined = &result.moisture_deduction_kg + &result.blank_deduction_kg;
        assert_eq!(combined, "110.000".parse::<BigDecimal>().unwrap());
    }

    #[test]
    fn mandatory_fields_are_really_enforced() {
        assert!(validate_mandatory_fields(&input(0, "0.2", "100", "0", "0", "0", 1)).is_err()); // bale_count
        assert!(validate_mandatory_fields(&input(1, "0.2", "0", "0", "0", "0", 1)).is_err()); // gross_weight
        assert!(validate_mandatory_fields(&input(1, "0.2", "100", "0", "0", "0", 0)).is_err()); // unit_price
        assert!(validate_mandatory_fields(&input(1, "0.15", "100", "0", "0", "0", 1)).is_err()); // bad tare_allowance
        assert!(validate_mandatory_fields(&input(1, "0.2", "100", "0", "0", "0", 1)).is_ok());
    }
}
