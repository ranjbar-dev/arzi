//! Step 6.6 (docs/phase-6-reporting.md §6.6): clean CSV/XLSX exports,
//! replacing the legacy's "export the rendered page" model (04-07.md §7.1's
//! FastReport filters carry the page furniture — letterhead, repeated
//! column headers, blank spacer rows — into the file, "awkward to
//! re-import"). `write_csv`/`write_xlsx` below are the two generic
//! renderers every export in this domain goes through — never a per-report
//! reimplementation of quoting or worksheet formatting.
//!
//! **CSV**: UTF-8, RFC 4180 quoting via the `csv` crate (never the
//! legacy's ANSI/Windows-1256, `UTF8 = False`, `OEMCodepage = False`
//! combination, 04-07.md §7.1) — a comma or newline inside a field is
//! quoted automatically, closing the exact defect the spec calls out
//! ("`ForcedQuotes = False` with `Separator = ','` — a description field
//! containing a comma breaks the row").
//!
//! **XLSX**: `rust_xlsxwriter`, per `10-target-architecture.md` §3.4's own
//! named choice — a clean data table, header row bold, no merged cells, no
//! repeated page headers, no blank spacer rows (04-07.md's "Done when").
//!
//! **B17 fix — the tax-authority export** (`get_tax_authority_export`
//! below, the `ToExcelDaraeiU` equivalent, 04-07.md §7.2): filters to
//! posted (`status = 'posted'`) vouchers only — the legacy's export "had no
//! `M_Tx` filter at all, so draft vouchers are exported to the tax
//! authority," which the spec calls "the most consequential instance of the
//! missing state filter in the whole system." Same `kind = 'ledger'` filter
//! every other report in this phase applies (matches the legacy's own
//! `M_kind = 1`, correctly present there — journal-summary rows never leak
//! into the tax file either way).
//!
//! **Also fixes the column-header defect found alongside B17**: the
//! legacy's first column is headed `ردیف` ("row number") but its data is
//! `M_Sanad`, the voucher number (04-07.md §7.2's "a real defect in a file
//! submitted to the tax authority"). The header here is `شماره سند`
//! ("voucher number") — labelled by what the column actually contains.
//!
//! **Text cleaning, done properly**: `collapse_whitespace` uses
//! `str::split_whitespace` — collapses *any* run of whitespace (including
//! the legacy's CRLF-then-single-double-space-pass sequence, which "does
//! not collapse runs of three or more spaces"), not a single find/replace
//! pass. No truncation at all: the legacy's 200-character cut existed only
//! because it was writing into a `Space(100)`-preallocated Excel-COM cell;
//! nothing about a `text/csv` or real `.xlsx` field forces a width limit,
//! so the "cuts mid-word" hazard (04-07.md §7.2's own note) cannot recur
//! here — there is no truncation left to apply.
//!
//! **B25 applied here too, unconditionally**: `fiscal_year_id` is a
//! required bind, same discipline as every other report in this phase.

use crate::{auth::authz, auth::AuthUser, db, AppState};
use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use chrono::NaiveDate;
use serde::Deserialize;
use serde_json::{json, Value};

pub fn router() -> Router<AppState> {
    Router::new().route("/tax-authority-export", get(get_tax_authority_export))
}

fn internal_error() -> (StatusCode, Json<Value>) {
    (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": "internal_error" })))
}
fn bad_request(error: &str) -> (StatusCode, Json<Value>) {
    (StatusCode::BAD_REQUEST, Json(json!({ "error": error })))
}

/// Collapses any run of whitespace (space, tab, CR, LF, ...) to a single
/// space and trims the ends — see module doc comment on why this is
/// strictly more correct than the legacy's one-pass `'  '` -> `' '` replace.
pub fn collapse_whitespace(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// RFC 4180 CSV, UTF-8, header row + data rows. `csv::Writer`'s default
/// quote style quotes any field containing the delimiter, a quote, or a
/// newline — the exact defect class 04-07.md §7.1 flags is structurally
/// closed by using a real CSV writer instead of a bare `Separator = ','`.
pub fn write_csv(columns: &[&str], rows: &[Vec<String>]) -> Result<String, csv::Error> {
    let mut writer = csv::Writer::from_writer(Vec::new());
    writer.write_record(columns)?;
    for row in rows {
        writer.write_record(row)?;
    }
    let bytes = writer.into_inner().map_err(|e| e.into_error())?;
    Ok(String::from_utf8(bytes).expect("csv writer only ever emits valid UTF-8 for UTF-8 input"))
}

/// A clean `.xlsx` data table: header row bold, no merged cells, no
/// repeated page headers, no blank spacer rows (04-07.md's "Done when").
/// Amount-looking columns (parseable as `i64`) are written as real numbers,
/// not text — a re-importable file, unlike `MoeinZipU`'s `.asstring`-
/// everywhere export (04-07.md §7.3).
pub fn write_xlsx(columns: &[&str], rows: &[Vec<String>]) -> Result<Vec<u8>, rust_xlsxwriter::XlsxError> {
    let mut workbook = rust_xlsxwriter::Workbook::new();
    let bold = rust_xlsxwriter::Format::new().set_bold();
    let sheet = workbook.add_worksheet();
    for (col, name) in columns.iter().enumerate() {
        sheet.write_string_with_format(0, col as u16, *name, &bold)?;
    }
    for (row_idx, row) in rows.iter().enumerate() {
        for (col_idx, value) in row.iter().enumerate() {
            let r = (row_idx + 1) as u32;
            let c = col_idx as u16;
            match value.parse::<f64>() {
                Ok(n) if !value.is_empty() => {
                    sheet.write_number(r, c, n)?;
                }
                _ => {
                    sheet.write_string(r, c, value)?;
                }
            }
        }
    }
    workbook.save_to_buffer()
}

fn csv_response(filename: &str, columns: &[&str], rows: &[Vec<String>]) -> Result<axum::response::Response, (StatusCode, Json<Value>)> {
    let body = write_csv(columns, rows).map_err(|_| internal_error())?;
    Ok((
        [
            (axum::http::header::CONTENT_TYPE, "text/csv; charset=utf-8".to_string()),
            (axum::http::header::CONTENT_DISPOSITION, format!("attachment; filename=\"{filename}.csv\"")),
        ],
        body,
    )
        .into_response())
}

fn xlsx_response(filename: &str, columns: &[&str], rows: &[Vec<String>]) -> Result<axum::response::Response, (StatusCode, Json<Value>)> {
    let body = write_xlsx(columns, rows).map_err(|_| internal_error())?;
    Ok((
        [
            (axum::http::header::CONTENT_TYPE, "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet".to_string()),
            (axum::http::header::CONTENT_DISPOSITION, format!("attachment; filename=\"{filename}.xlsx\"")),
        ],
        body,
    )
        .into_response())
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct TaxAuthorityExportQuery {
    fiscal_year_id: i64,
    from_date: NaiveDate,
    to_date: NaiveDate,
    #[serde(default = "default_format")]
    format: String,
}
fn default_format() -> String {
    "csv".to_string()
}

#[derive(sqlx::FromRow)]
struct TaxRow {
    voucher_number: i32,
    line_date: NaiveDate,
    kol_code: i32,
    kol_name: String,
    moein_code: i32,
    moein_name: Option<String>,
    description: Option<String>,
    debit_amount: i64,
    credit_amount: i64,
}

async fn get_tax_authority_export(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(params): Query<TaxAuthorityExportQuery>,
) -> Result<axum::response::Response, (StatusCode, Json<Value>)> {
    authz::require(&auth, "tax_authority_export")?;
    if params.from_date > params.to_date {
        return Err(bad_request("date_range_inverted"));
    }
    if params.format != "csv" && params.format != "xlsx" {
        return Err(bad_request("invalid_format"));
    }
    let mut tx = db::begin(&state.pool, auth.tenant_id).await.map_err(|_| internal_error())?;

    let rows: Vec<TaxRow> = sqlx::query_as(
        "SELECT v.voucher_number, vl.line_date, \
                a.general_ledger_code AS kol_code, ka.name AS kol_name, \
                a.subsidiary_code AS moein_code, ma.name AS moein_name, \
                vl.description, vl.debit_amount, vl.credit_amount \
         FROM voucher_lines vl \
         JOIN vouchers v ON v.id = vl.voucher_id \
         JOIN accounts a ON a.id = vl.account_id \
         JOIN accounts ka ON ka.tenant_id = a.tenant_id AND ka.general_ledger_code = a.general_ledger_code \
              AND ka.subsidiary_code = 0 AND ka.analytic1_code = 0 AND ka.analytic2_code = 0 \
         LEFT JOIN accounts ma ON ma.tenant_id = a.tenant_id AND ma.general_ledger_code = a.general_ledger_code \
              AND ma.subsidiary_code = a.subsidiary_code AND ma.analytic1_code = 0 AND ma.analytic2_code = 0 \
         WHERE vl.tenant_id = $1 AND vl.fiscal_year_id = $2 AND vl.kind = 'ledger' AND vl.status = 'posted' \
           AND vl.line_date >= $3 AND vl.line_date <= $4 \
         ORDER BY vl.line_date, v.voucher_number, vl.id",
    )
    .bind(auth.tenant_id)
    .bind(params.fiscal_year_id)
    .bind(params.from_date)
    .bind(params.to_date)
    .fetch_all(&mut *tx)
    .await
    .map_err(|_| internal_error())?;
    tx.rollback().await.ok();

    // "شماره سند" (voucher number), not "ردیف" (row number) -- the column-
    // header fix (see module doc comment).
    let columns = ["شماره سند", "تاریخ", "کل", "نام کل", "معین", "نام معین", "شرح", "مبلغ بدهکار", "مبلغ بستانکار"];
    let data_rows: Vec<Vec<String>> = rows
        .iter()
        .map(|r| {
            vec![
                r.voucher_number.to_string(),
                r.line_date.to_string(),
                r.kol_code.to_string(),
                r.kol_name.clone(),
                r.moein_code.to_string(),
                r.moein_name.clone().unwrap_or_default(),
                collapse_whitespace(r.description.as_deref().unwrap_or("")),
                r.debit_amount.to_string(),
                r.credit_amount.to_string(),
            ]
        })
        .collect();

    if params.format == "xlsx" {
        xlsx_response("tax-authority-export", &columns, &data_rows)
    } else {
        csv_response("tax-authority-export", &columns, &data_rows)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collapse_whitespace_handles_runs_of_three_or_more() {
        assert_eq!(collapse_whitespace("a   b"), "a b");
        assert_eq!(collapse_whitespace("  a\r\nb\t c  "), "a b c");
        assert_eq!(collapse_whitespace(""), "");
    }

    #[test]
    fn csv_quotes_a_comma_and_never_breaks_the_row() {
        let out = write_csv(&["a", "b"], &[vec!["hello, world".to_string(), "x".to_string()]]).unwrap();
        let mut reader = csv::Reader::from_reader(out.as_bytes());
        let record = reader.records().next().unwrap().unwrap();
        assert_eq!(&record[0], "hello, world");
        assert_eq!(&record[1], "x");
    }

    #[test]
    fn xlsx_renders_real_bytes() {
        let bytes = write_xlsx(&["a", "b"], &[vec!["1".to_string(), "text".to_string()]]).unwrap();
        assert!(bytes.starts_with(b"PK"), "an xlsx is a zip archive, starts with the PK signature");
    }
}
