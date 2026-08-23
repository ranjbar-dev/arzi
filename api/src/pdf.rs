//! Step 6.5 (docs/phase-6-reporting.md §6.5): server-side, RTL-correct PDF
//! generation, replacing FastReport, which the legacy used for every
//! printable surface with no PDF output at all (04-06-a.md §6's opening
//! line: "no `TPrinter` drawing, no HTML, no PDF library").
//!
//! **Engine choice**: [Typst](https://typst.app), embedded in-process via
//! `typst-as-lib` — pure Rust, no sidecar process, real bidi + font shaping
//! (Typst uses `rustybuzz`, a Rust port of HarfBuzz, so Persian letter
//! joining and mixed Persian/Latin digit direction are handled by the
//! shaping engine, not approximated by hand). Templates are `include_str!`
//! constants compiled into the binary (`templates/*.typ`) and fonts are
//! `include_bytes!` (`assets/fonts/Vazirmatn-{Regular,Bold}.ttf`, the same
//! family the web frontend already uses, 1.6's own choice) — **this closes
//! B23 structurally**: there is no route anywhere in this API that accepts
//! a `.typ`/`.fr3` file or lets a caller choose a template, so "any user who
//! can open any preview can edit the report layout" (04-06-a.md §6.2's
//! defect) cannot be reproduced even by accident.
//!
//! **One structured document header** (`PrintHeader`), consumed by every
//! template — replaces the legacy's 15+ string-literal memo names matched
//! by convention (`T1`, `T2`, `_Total`, `B3`, ... — 04-06-a.md §6.1's "data
//! binding is by convention, not by contract" warning). Signature blocks
//! (`Tanzim` 1011-1014, 04-06-a.md §6.5) are read from `app_settings`
//! (already the rebuild's Tanzim equivalent, seeded per-tenant since 1.1's
//! migration but never yet populated by any step) with the missing-key
//! default fixed to an **empty string**, not the legacy's "getter that
//! writes the label as the value" self-healing bug (§6.5's "fresh
//! installations print سند امضا ۴ where a signature block should be") —
//! `fetch_signature_labels` below never writes on read.
//!
//! **Scope, a documented boundary, not a silent narrowing**: the Build
//! bullet names "vouchers, invoices, ledgers, trial balances, cheque/petty-
//! cash documents." Two routes are wired this step: `GET
//! /vouchers/{id}/pdf` (the voucher template, `templates/voucher.typ`) and
//! `GET /reports/trial-balance-4-column/pdf` (the generic tabular-report
//! template, `templates/tabular_report.typ`, proving both the letterhead
//! and the amount-in-words footer end to end per the manual test). Cheque,
//! deposit-slip and petty-cash documents already post through the Phase 2.5
//! voucher engine and carry a real `voucher_id` (4.2/4.4) — "print a cheque
//! receipt" is "print its linked voucher" through the SAME endpoint, not a
//! bespoke template, so that item is covered without new code. Ledgers, the
//! 6-column trial balance, the party-balance list and invoice printing are
//! the SAME tabular shape (`tabular_report.typ` is written generically for
//! exactly this reuse) but their own PDF routes are not wired here — left
//! for whoever next needs that specific report as a download, matching this
//! codebase's own repeated precedent (2.5's `auto_post` engine shipped
//! generic with its first real caller deferred to a later step).
//!
//! **Amount-in-words** is `persian_words::amount_in_words` — a real
//! function over `i64`, not `Dm.Str2String`'s string-sliced port (see that
//! module's own doc comment).

use crate::persian_words::amount_in_words;
use derive_typst_intoval::{IntoDict, IntoValue};
use sqlx::{Postgres, Transaction};
use std::sync::OnceLock;
use typst::foundations::{Dict, IntoValue};
use typst_as_lib::{TypstEngine, TypstTemplateMainFile};

const VAZIRMATN_REGULAR: &[u8] = include_bytes!("../assets/fonts/Vazirmatn-Regular.ttf");
const VAZIRMATN_BOLD: &[u8] = include_bytes!("../assets/fonts/Vazirmatn-Bold.ttf");
const VOUCHER_TEMPLATE: &str = include_str!("../templates/voucher.typ");
const TABULAR_TEMPLATE: &str = include_str!("../templates/tabular_report.typ");

fn voucher_engine() -> &'static TypstEngine<TypstTemplateMainFile> {
    static ENGINE: OnceLock<TypstEngine<TypstTemplateMainFile>> = OnceLock::new();
    ENGINE.get_or_init(|| {
        TypstEngine::builder()
            .main_file(VOUCHER_TEMPLATE)
            .fonts([VAZIRMATN_REGULAR, VAZIRMATN_BOLD])
            .build()
    })
}

fn tabular_engine() -> &'static TypstEngine<TypstTemplateMainFile> {
    static ENGINE: OnceLock<TypstEngine<TypstTemplateMainFile>> = OnceLock::new();
    ENGINE.get_or_init(|| {
        TypstEngine::builder()
            .main_file(TABULAR_TEMPLATE)
            .fonts([VAZIRMATN_REGULAR, VAZIRMATN_BOLD])
            .build()
    })
}

#[derive(Debug, thiserror::Error)]
pub enum PdfError {
    #[error("typst compilation failed: {0}")]
    Compile(String),
    #[error("pdf export failed: {0}")]
    Export(String),
}

/// The one document-header object every template consumes (see module doc
/// comment). `period_caption`/`amount_in_words` are optional since not
/// every document has a period range (a single voucher doesn't) or a
/// headline total (a detail-only ledger listing might not).
#[derive(Clone, IntoValue, IntoDict)]
pub struct PrintHeader {
    pub organization_name: String,
    pub fiscal_year_caption: String,
    pub report_title: String,
    pub period_caption: Option<String>,
    pub amount_in_words: Option<String>,
    /// Up to four configurable signature-block labels (`Tanzim` 1011-1014
    /// for vouchers, 1013/1014 shared with ledgers/trial-balances per
    /// 04-06-a.md §6.5's own table) — plain strings, never template edits.
    pub signature_labels: Vec<String>,
}

/// The letterhead's organisation name (04-06-a.md §6.4's `DM.RegName`,
/// loaded from `Base.CO_Name`) — falls back to the tenant's own name when no
/// `organization` row has been created yet (1.1/1.6 never seeded one; same
/// "no tenant-provisioning flow exists" gap already documented elsewhere in
/// this codebase for `account_code_format`/`party_account_config`).
pub async fn fetch_organization_name(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: i64,
) -> Result<String, sqlx::Error> {
    sqlx::query_scalar(
        "SELECT COALESCE( \
             (SELECT name FROM organization WHERE tenant_id = $1), \
             (SELECT name FROM tenants WHERE id = $1) \
         )",
    )
    .bind(tenant_id)
    .fetch_one(&mut **tx)
    .await
}

/// `Set_paramstr`/`Get_paramstr`'s rebuild equivalent, minus the "getter
/// that writes" bug: a missing key is simply an empty string, and this
/// function never inserts a row. `keys` in display order (1011..1014 or
/// whichever subset a document type actually uses).
pub async fn fetch_signature_labels(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: i64,
    keys: &[&str],
) -> Result<Vec<String>, sqlx::Error> {
    let mut labels = Vec::with_capacity(keys.len());
    for key in keys {
        let value: Option<String> =
            sqlx::query_scalar("SELECT value FROM app_settings WHERE tenant_id = $1 AND key = $2")
                .bind(tenant_id)
                .bind(key)
                .fetch_optional(&mut **tx)
                .await?;
        labels.push(value.unwrap_or_default());
    }
    Ok(labels)
}

#[derive(Clone, IntoValue, IntoDict)]
struct VoucherLineInput {
    account_label: String,
    description: String,
    /// Pre-formatted, thousands-grouped, empty string when zero (B6.8:
    /// "zero renders as blank" — the template is purely presentational,
    /// business formatting happens here where it's testable).
    debit: String,
    credit: String,
}

#[derive(Clone, IntoValue, IntoDict)]
struct VoucherInput {
    header: PrintHeader,
    voucher_number: String,
    voucher_date: String,
    description: String,
    lines: Vec<VoucherLineInput>,
    total_debit: String,
    total_credit: String,
    amount_in_words: String,
}

impl From<VoucherInput> for Dict {
    fn from(value: VoucherInput) -> Self {
        value.into_dict()
    }
}

/// `#,###` grouping, empty string for zero (04-06-b.md §6.8's grid/report
/// convention, applied uniformly here rather than the legacy's two
/// disagreeing formatters).
fn format_amount(amount: i64) -> String {
    if amount == 0 {
        return String::new();
    }
    let negative = amount < 0;
    let digits = amount.unsigned_abs().to_string();
    let mut grouped = String::new();
    for (i, c) in digits.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            grouped.push(',');
        }
        grouped.push(c);
    }
    let grouped: String = grouped.chars().rev().collect();
    if negative {
        format!("-{grouped}")
    } else {
        grouped
    }
}

pub struct VoucherLineData {
    pub account_label: String,
    pub description: String,
    pub debit: i64,
    pub credit: i64,
}

/// Renders one voucher (`SanadEditU`/`PrintNu`/`PrintM2U` equivalent — see
/// module doc comment on why this also covers cheque/petty-cash prints).
pub fn render_voucher_pdf(
    header: PrintHeader,
    voucher_number: i32,
    voucher_date: &str,
    description: &str,
    lines: &[VoucherLineData],
    total_debit: i64,
    total_credit: i64,
) -> Result<Vec<u8>, PdfError> {
    let input = VoucherInput {
        header,
        voucher_number: voucher_number.to_string(),
        voucher_date: voucher_date.to_string(),
        description: description.to_string(),
        lines: lines
            .iter()
            .map(|l| VoucherLineInput {
                account_label: l.account_label.clone(),
                description: l.description.clone(),
                debit: format_amount(l.debit),
                credit: format_amount(l.credit),
            })
            .collect(),
        total_debit: format_amount(total_debit),
        total_credit: format_amount(total_credit),
        // The total is always the balanced (equal) figure, per 03-03/2.3's
        // own invariant on a non-draft voucher -- either side is correct.
        amount_in_words: amount_in_words(total_debit),
    };
    compile_and_export(voucher_engine(), input)
}

#[derive(Clone, IntoValue, IntoDict)]
struct TabularReportInput {
    header: PrintHeader,
    columns: Vec<String>,
    rows: Vec<Vec<String>>,
    totals: Option<Vec<String>>,
}

impl From<TabularReportInput> for Dict {
    fn from(value: TabularReportInput) -> Self {
        value.into_dict()
    }
}

/// Renders any report matching the generic tabular shape (columns already
/// supplied in right-to-left display order; each row/`totals` a flat vec of
/// pre-formatted cell strings — same "format in Rust, render in Typst"
/// split as `render_voucher_pdf`).
pub fn render_tabular_report_pdf(
    header: PrintHeader,
    columns: Vec<String>,
    rows: Vec<Vec<String>>,
    totals: Option<Vec<String>>,
) -> Result<Vec<u8>, PdfError> {
    let input = TabularReportInput {
        header,
        columns,
        rows,
        totals,
    };
    compile_and_export(tabular_engine(), input)
}

fn compile_and_export<D: Into<Dict>>(
    engine: &TypstEngine<TypstTemplateMainFile>,
    input: D,
) -> Result<Vec<u8>, PdfError> {
    let doc = engine
        .compile_with_input(input)
        .output
        .map_err(|e| PdfError::Compile(format!("{e:?}")))?;
    typst_pdf::pdf(&doc, &Default::default()).map_err(|e| PdfError::Export(format!("{e:?}")))
}

/// Public so callers building a report row can use the identical formatter
/// this module uses internally (never a second, drifting implementation).
pub fn format_amount_public(amount: i64) -> String {
    format_amount(amount)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_amount_groups_and_blanks_zero() {
        assert_eq!(format_amount(0), "");
        assert_eq!(format_amount(1234567), "1,234,567");
        assert_eq!(format_amount(-500), "-500");
        assert_eq!(format_amount(999), "999");
    }

    fn sample_header() -> PrintHeader {
        PrintHeader {
            organization_name: "Acme Co".to_string(),
            fiscal_year_caption: "سال مالی ۱۴۰۶".to_string(),
            report_title: "سند حسابداری".to_string(),
            period_caption: Some("از ۱۴۰۶/۰۱/۰۱ تا ۱۴۰۶/۱۲/۲۹".to_string()),
            amount_in_words: Some(amount_in_words(1_500_000)),
            signature_labels: vec!["تنظیم کننده".to_string(), "مدیر مالی".to_string()],
        }
    }

    /// The actual proof the engine works, not just that the Rust side
    /// compiles: a real Typst compile + PDF export, with RTL Persian text
    /// (organisation name, description, amount-in-words) in the input.
    #[test]
    fn voucher_pdf_renders_real_bytes() {
        let lines = vec![
            VoucherLineData {
                account_label: "10-1".to_string(),
                description: "نقد".to_string(),
                debit: 1_500_000,
                credit: 0,
            },
            VoucherLineData {
                account_label: "40-1".to_string(),
                description: "فروش".to_string(),
                debit: 0,
                credit: 1_500_000,
            },
        ];
        let pdf = render_voucher_pdf(
            sample_header(),
            42,
            "1406/06/01",
            "سند نمونه",
            &lines,
            1_500_000,
            1_500_000,
        )
        .expect("voucher pdf must render");
        assert!(pdf.starts_with(b"%PDF-"), "output must be a real PDF");
        assert!(
            pdf.len() > 500,
            "a real rendered page should not be a near-empty stub"
        );
    }

    #[test]
    fn tabular_report_pdf_renders_real_bytes() {
        let pdf = render_tabular_report_pdf(
            sample_header(),
            vec![
                "نام".to_string(),
                "بدهکار".to_string(),
                "بستانکار".to_string(),
            ],
            vec![
                vec![
                    "Assets".to_string(),
                    "1,500,000".to_string(),
                    "".to_string(),
                ],
                vec![
                    "Revenue".to_string(),
                    "".to_string(),
                    "1,500,000".to_string(),
                ],
            ],
            Some(vec![
                "جمع".to_string(),
                "1,500,000".to_string(),
                "1,500,000".to_string(),
            ]),
        )
        .expect("tabular report pdf must render");
        assert!(pdf.starts_with(b"%PDF-"));
        assert!(pdf.len() > 500);
    }
}
