_Part of [04-reporting](../04-reporting.md) — [index](00-index.md)_

## 8. Rebuild recommendations

Target: Rust backend, React frontend, PostgreSQL, Docker; SQLite/in-memory for tests. Business logic
preserved exactly (deviations only where §10 is approved). **No code below — approach only.**

### 8.0 Decisions that apply to every report

**One posting-lines query builder, not thirty string-concatenated queries.** Every report in §1–§4
reduces to the same shape: filter `voucher_lines` by `(fiscal_year, date_range | as_of,
account_predicate, line_source, voucher_states)`, aggregate, present. Build one typed filter struct
and one query builder; each report is a projection over it. That single change removes the entire
class of bugs documented above (missing `M_kind`, missing `M_Tx`, `M_Ko=0'+text`, empty-`WHERE` syntax
errors, SQL fragments passed between forms as strings).

**Test the SQLite path.** The legacy queries lean on SQL Server specifics — `SELECT … INTO #temp`,
`Object_ID('tempdb..#R')`, `ROW_NUMBER()` mixed with correlated subqueries, `Sign()`, `Space()`,
`Cast(x as Bigint)`, scalar UDFs. None of that survives a Postgres/SQLite dual target. Express every
report as **CTEs plus window functions** — portable across both — and never as temp tables.

**Money type.** All amounts are integral rials in `bigint`. Use `i64` end to end and
`numeric(20,0)` in Postgres; never `f64`, never `NUMERIC` with a scale. Quantities (`M_Ted`) are
`decimal(18,3)` and need `rust_decimal`.

**Date model.** Store `entry_date DATE` (Gregorian) *and* `entry_date_jalali CHAR(10)`. Filter and
sort on the `DATE`; render the string. Convert once, at write time, with a pinned Jalali library and
a golden-file test over the full range of years present in the data (§5.1).

**Ordering.** Every listing orders by `(entry_date, voucher_no, line_id)`. The third key is new and
non-negotiable — the legacy `(M_Date, M_Sanad)` is non-deterministic within a voucher (§3.1.c).

**Balance representation.** Compute one signed `balance: i64` (debit-positive is the recommended
convention — note the legacy is credit-positive) and derive `{ debit, credit }` or
`{ amount, side }` at the presentation layer. Do not carry two clamped columns through the domain.

**Voucher state and line source are first-class filters on every endpoint**, defaulted explicitly per
report to whatever §1–§4 records as the legacy behaviour, and always echoed back in the response so
the UI can display "includes drafts".

**Rendering.** A single `<ReportShell>` React component supplying: RTL layout, the filter bar, the
sticky repeating header, odd-row banding, zero-as-blank number formatting, the `بد`/`بس` side
indicator, the signature block, print CSS (`@page { size: A4 portrait; margin: 10mm }`,
`thead { display: table-header-group }`), and CSV/XLSX download. Individual reports supply a column
spec and a row renderer only. This replaces 58 hand-drawn FastReport layouts.

**Print and PDF.** Print from the browser with print CSS for everyday use; render server-side
(headless Chromium or `typst`/`weasyprint`) for archival PDFs. The legacy system has no PDF at all
(§7.5) — adding one is a strict improvement and does not change business logic.

**Permissions.** Replace the flat `Pass_Config` `(user, key)` integer table with named permissions
checked in a middleware layer, and close the gaps recorded in §1.0 and §4.7 (five ungated report menu
items; `Report2` ungated in Card Jari).

### 8.1 Per report

Legend for the data strategy: **Q** = plain SQL query per request; **MV** = materialised view
refreshed on posting; **AGG** = maintained aggregate table.

#### R01 `trial_balance_4col` (§2.1)
**Q.** One CTE that aggregates `voucher_lines` to the deepest requested level, then `GROUPING SETS`
(or `ROLLUP`) to emit the shallower levels in the same pass — replacing the four separate `INSERT`s
into `#R`. Emit an explicit `level` column so the client can indent instead of the four-spaces hack,
and so the grand total filters on `level = 1` as the legacy `IIF(St > 1, 0, 1)` does. Add the
**mandatory balance assertion** (`Σ debit = Σ credit` over level 1) and surface the difference; the
legacy silently absorbs it. Cheap enough for Q at any realistic year size (single table scan,
grouped).
**React:** one flat table, `level` driving indent and row tint, sticky header, footer totals from the
API not from the client. Level selector re-fetches.

#### R02 `trial_balance_6col` (§2.2)
**Q**, and **rewrite rather than port** — the stored-procedure body is not available (§9). Express it
as two aggregate CTEs over the same line set: one with `entry_date < from_date` (opening turnover),
one with `from_date <= entry_date <= to_date` (period turnover), joined on account, with the closing
pair derived. Then run both engines side by side against a live database and reconcile before cutting
over. Merge with R01 behind one endpoint carrying an `as_of` vs `from..to` mode switch, exactly as
§2.3 recommends.
**React:** the same table component with a two-tier column header group (`گردش قبل از دوره` /
`گردش طی دوره` / `مانده پایان دوره`) rendered as a second `<tr>` in `<thead>` with `colSpan`.

#### R03/R04/R05 `general_ledger` / `subsidiary_ledger` (§3.1–§3.3)
**Q**, one endpoint, three presets. Structure: a CTE for the opening aggregate
(`entry_date < from_date`, returning two gross sums *and* the net), a CTE for the movement rows, then
`SUM(credit - debit) OVER (ORDER BY entry_date, voucher_no, line_id ROWS UNBOUNDED PRECEDING)` for the
running balance — replacing the O(n²) correlated subquery. The opening row becomes a typed
`openingRow` object in the response, not a fake data row with a magic description.
`line_source` is a parameter (`Subsidiary` for R04/R05, `JournalSummary` for R03) applied to **both**
legs — fixing the §3.3(b) double-count. Account selection is a structured filter, collapsing R05 into
R04.
**React:** virtualised table (a ledger can run to tens of thousands of rows), sticky header, a pinned
opening row, running-balance column showing `abs(value)` plus a `بد`/`بس` chip, and click-through to
the voucher. Add a client-side "recompute from this row" affordance so the running balance is
auditable.

#### R06 `ledger_multi_account` (§3.4)
**Q.** Account multi-select at one level → `account_id = ANY($1)` (or a subtree predicate for the
level-1 case, which is what `M_Ko in (…)` really means). Add the opening balance and running balance
this report lacks — or keep it deliberately as a flat listing and say so in the UI. Fix the level
titles, which are currently off by one (§3.4).
**React:** two-pane layout — a lazy account tree on one side, the result table on the other; replace
the four-state `_State` machine with a real tree component and breadcrumbs.

#### R07 `account_monthly_turnover` (§3.5)
**MV** or **Q**. Twelve rows per account per year: `date_trunc`-equivalent on the Jalali month, i.e.
`substring(entry_date_jalali, 6, 2)`, grouped. Trivial as Q; a small **MV keyed
`(fiscal_year, account_id, jalali_month)`** is attractive because the same aggregate feeds the
6-column trial balance's period columns and any future dashboard. Requires the `KolState;1` body to
confirm semantics (§9) — the shape (`Sal`, `mahstr`, `M_Bed`, `M_Bes`) is known, the filter is not.
Add the date range the legacy lacks, and the column totals its report has none of.
**React:** twelve-row table plus a small grouped bar chart (debit/credit per month). This is the one
report where a chart genuinely helps.

#### R08 `party_account_summary` (§4)
**Q, single round trip.** The legacy issues `2n + 3` queries (§4.4). Replace with one query: expand
the `SahamdarConfig` templates for the party in a CTE, join to `accounts`, left-join a grouped
aggregate of `voucher_lines`. Return both figures the screen shows — the per-account rows and the
`SC_Rem = 1` restricted `finalBalance` — from **one** aggregate so they cannot disagree, and label
them so the difference is explicable. Recompute `accountCode` rather than reading the stale
`Sarfasl.M_R` (§4.4). Fix the permission ordering so the lock check precedes the identity payload.
**React:** identity card + photo (behind an authorised object-storage URL), then a table with a
footer row, then two buttons: "ledger" (current row) and "consolidated ledger" (selection). Selection
by checkbox, not by double-click; double-click opens the ledger, matching every other grid.

#### R09 `party_balance_list` (§1.2)
**Q**, and this is the one report where **AGG** may become necessary: it aggregates the entire
`voucher_lines` table for a year and then re-groups by party. If it is slow, maintain
`party_balances(fiscal_year, party_id, opening, debit, credit, closing)` incrementally on posting.
Align the opening boundary with the ledgers (`< from_date`, not `<= from_date`) — see §10, this
changes numbers. Drop the hard-coded 1 000 000 / 100 000 000 amount defaults. Add sorting by amount.
Add the missing `Rem1` column total.
**React:** sortable table (default: closing balance descending, which is what users actually want),
debtors/creditors toggle, amount range as an optional filter that is empty by default, click-through
to the party summary.

#### R10 `voucher_number_gap_check` (§1.3)
**Q.** Two statements: the per-voucher aggregate, and a `generate_series(from, to) EXCEPT SELECT
DISTINCT voucher_no …` for the gaps — replacing the client-side `Locate` loop and its uninitialised
counter. Read the voucher description from `vouchers` (the header), not `min(line.description)`.
Keep `min(state)` as the displayed voucher state; it is the conservative choice.
**React:** one table plus a prominent "N missing: 12, 15, 31–34" summary chip with range collapsing.

#### R11 `account_turnover_explorer` (§1.4)
**Q with CTEs — and delete the per-user table.** `temp_RJ_<userId>` must not survive the port
(§10.7). The whole pipeline is one statement: filter → aggregate to leaves → `GROUPING SETS` for the
three roll-up levels → clamp → join names. If interactive drill-down proves slow on the full chart of
accounts, cache the result set in the API layer keyed by the filter hash, not in the database.
Preserve `isLeaf`, which is a better anti-double-count device than the trial balance's `St`. Fix the
hard-coded `Dbo.Make_R(1, …)` fiscal-year argument. Drop the print-follows-grid-widths behaviour
(§6.9) unless users specifically ask for it.
**React:** an expandable tree table (row expansion fetching or filtering children), replacing the
four-mode radio group with a single "expand to level" control plus free expansion.

#### R13 `voucher_summary` (§1.6)
**Q.** The four side-split queries collapse to one with a `side` parameter, or to one query returning
both sides. Keep `M_kind = 1`. Keep "export current view" (§7.3) as a first-class feature — it is the
most useful thing in the unit — but produce a real `.xlsx` file with typed cells, not an unsaved
workbook of strings.
**React:** column-picker driven table; the export endpoint takes the same column list.

#### R14 `journal_voucher_list` (§1.7)
**Q** against `vouchers` where `kind = JournalSummary`, but **derive the totals from
`voucher_lines`** rather than reading the cached `DM_TBed`/`DM_TBes`; the cache is drift-prone and
this screen is the only place it is displayed as authority. Add a secondary sort key. The print of a
single voucher becomes the shared `voucher_print` endpoint. The edit actions (date, number,
description, post, lock, delete) belong to `03-accounting-core.md` and must go through the domain
layer, not through two-table `UPDATE`s — in particular the date change must scope to the voucher's
own lines, which the legacy does not (§1.7).
**React:** master list + detail panel; actions as explicit mutations with optimistic invalidation.

#### R16–R20 `voucher_print` (§1.9, §6)
**Q.** One endpoint returning a voucher header plus lines plus resolved account names plus the
amount-in-words. Three template variants selected by a **setting**, not by editing source. The
"fast print" path becomes "print without preview" in the client. The `PrintMU` per-row frame and
colour mutation (`RP2GetValue`, §6.3) is a Kol-header/Moein-detail visual grouping — express it as a
grouped table, and note that `PrintMU` is dead so this only matters if the layout is being revived.

#### R22 `chart_of_accounts` print (§1.10)
**Q**, recursive CTE over the account tree. Belongs to `03-accounting-core.md`; only the printable
listing is in scope here.

#### R25 `voucher_export_tax_authority` (§7.2)
**Q**, and it needs its own scrutiny before anything else: it exports **draft vouchers**, it labels
voucher numbers as `ردیف`, and it drops the analytic levels. Rebuild as a dedicated, versioned export
with an explicit `states` parameter (default: posted only — see §10), a validated column contract
against the authority's template, and a stored record of every export run (who, when, what range,
what checksum). Generate `.xlsx` server-side with `rust_xlsxwriter`; no Excel installation required.
**React:** a small wizard — period, state filter, preview of the first 50 rows, row count and control
totals, then download.

### 8.2 Reports to delete rather than port

`Report7U` (§1.5), `ListSarfaslu`, `S_KolU`, `Lab`, `ToExcelU`, `LibXL`, `RoozViewU`, `KolSatateU`,
`SarfaslChap`, `AnbarReportKharidU`, `Taraz4Setooni_U.RP_Kol1`, `DM.SP_Taraz4Setooni`,
`Mainu._Report9`/`TajmiU`, `Mainu.SRooz1/2/3/4`, `PrintMU`, `PrintM2U.Print1`,
`MoeinZipU.Button1Click`, and every dead control listed at the end of §1.13. Confirm with the
customer that nothing on this list is used before deleting — several are dead only because a single
line was commented out and could be revived by uncommenting it.

### 8.3 Suggested build order

1. The shared posting-lines query builder + the date/fiscal-year model (§5) + `ReportShell`.
2. `subsidiary_ledger` (R04/R05) — highest use, and it exercises opening balance, running balance,
   ordering and drill-down all at once.
3. `general_ledger` (R03) — same engine, different `line_source`; forces the §3.0 journal-summary
   question to be resolved early.
4. `trial_balance` (R01+R02 merged) with the balance assertion, reconciled against the legacy.
5. `party_account_summary` (R08) and `party_balance_list` (R09) — shared `SahamdarConfig` expansion.
6. `voucher_print` and the print/PDF layer.
7. `voucher_export_tax_authority` (R25) — needs the state decision settled first.
8. `account_turnover_explorer` (R11), `voucher_summary` (R13), the control reports (R10, R07).


---

[← SS7 Export pipeline](04-07-export-pipeline.md) | [Index](00-index.md) | [SS9 Open questions →](04-09-open-questions.md)
