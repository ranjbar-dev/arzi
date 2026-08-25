# Phase 6 — Reporting

Every report reads data created in Phases 2–5. This phase is where the legacy's silent
out-of-balance reports, the ledger's opening-balance bugs, and the security-relevant "everything
exportable, nothing gated" defaults all get closed.

---

## 6.1 Trial balances

**Goal:** 4-column and 6-column trial balances, posted-only (A8), with the legacy's silent
out-of-balance risk closed.

**Build**

- 4-column trial balance (`Taraz4Setooni_U`): cumulative debit/credit since fiscal-year start up to
  a chosen date, plus the netted balance pair (`balance_debit = max(cumulative_debit −
  cumulative_credit, 0)`, and the credit side symmetrically) — the exact netting rule from
  `04-02-a.md` §2.1. Support the four levels (Kol / +Moein / +Tafsil1 / +Tafsil2), interleaved with
  correct indentation, matching the legacy's level semantics but computed cleanly (real recursive/
  grouped SQL, not a hand-built temp-table with three nearly-identical `INSERT` blocks).
- **A8 fix, applied here**: filter to posted (`status = 2`) vouchers only, always — no equivalent of
  the legacy's three voucher-state checkboxes that were wired to nothing (`04-02-a.md` §2.1's defect
  (a): "the 4-column trial balance therefore always includes vouchers in every state, including
  unposted drafts," despite UI controls that implied otherwise). Don't build dead checkboxes; if a
  draft/posted toggle is wanted later, it must actually filter.
- **New: an explicit balance-proof.** The legacy "never checks this and never displays a difference"
  when debits don't equal credits across the year (`04-02-a.md` §2.1's "single most important defect
  to fix" callout) — the rebuild computes Σdebit and Σcredit at Kol level and asserts they're equal,
  surfacing a hard error banner if they aren't, rather than silently absorbing a discrepancy into two
  clamped columns. This should be structurally rare given Phase 2's balance-on-issue enforcement, but
  the report must prove it, not assume it.
- Fiscal-year selector actually scopes the query (fixing the legacy's dropdown-that-doesn't-filter
  bug, `04-02-a.md` §2.1 defect (b) — `@Co` only affected code-string formatting, never which rows
  were read).
- 6-column trial balance: read `04-02-b-trial-balances-in-depth.md` before implementing (opening /
  period / closing columns) — not detailed further here since it wasn't covered in this pass; apply
  the same A8 posted-only filter and the same balance-proof principle.

**Spec refs:** `04-02-a/b-trial-balances-in-depth.md`; A8 in `11-open-decisions.md`.

**Manual test**

1. Post several balanced vouchers across two Kol accounts, leave one voucher as a draft → run the
   4-column trial balance → confirm the draft voucher's amounts are excluded (A8 fix).
2. Confirm Σdebit = Σcredit at Kol level is displayed as proven, not silently assumed.
3. Change the fiscal-year selector → confirm the report actually re-scopes to that year's data (not
   just its code-string formatting, unlike the legacy).
4. Run at each of the four detail levels → confirm indentation and interleaving are correct and the
   grand total never double-counts (matches Kol-level total regardless of chosen detail level).

**Done when:** drafts never appear in a trial balance, and an out-of-balance condition — however it
might arise — is surfaced, never silently absorbed.

---

## 6.2 General & subsidiary ledgers

**Goal:** Daftar Kol and Daftar Moein, with the opening-balance and prerequisite bugs fixed (B4, B5,
B6).

**Build**

- **B6 fix — this is the core structural change.** The legacy's general ledger (`DKolU`) reads only
  `M_Kind = 2` rows — lines of a *manually generated* journal-summary voucher — so "Daftar Kol shows
  nothing until someone presses 'ساخت روزنامه'" and even then only shows draft-state summary rows
  (`04-03-a.md` §3.0's consequences list, items 1 and 3). The rebuild's general ledger reads posted
  (`status = 2`) voucher lines **directly**, grouped/rolled up to Kol level on the fly — journal
  (Rooznameh) generation was never built, so there is nothing for the ledger to depend on.
- Subsidiary ledger (Daftar Moein): one 4-segment account, full transaction detail, opening balance
  + running balance — matching `04-03-a.md` §3.1's shape but for `M_kind=1`-equivalent (ordinary
  posted lines) directly, not the summary-only path.
- **Opening balance — computed correctly, once.** `Σcredit − Σdebit` for all posted lines strictly
  before the from-date (matching the legacy's correct `M_Date < D1` boundary — no off-by-one there,
  `04-03-a.md` §3.1.a — keep that part). Net it into one signed figure; do **not** reproduce the
  legacy's "two gross totals side by side, no netting" quirk on the opening row itself — that was
  cosmetic confusion, not a feature worth preserving.
- **B4 fix**: the consolidated/aggregate ledger's opening leg must use the same inclusion rule as
  every other ledger — the legacy's version "omits `M_Kind=1`" on the opening leg while including it
  on the movement leg, double-counting the opening balance (`11-open-decisions.md` B4). Use one
  consistent predicate for both legs.
- **B5 fix**: use `< from_date` (strictly less than) as the opening/period boundary everywhere,
  including in the party-balance report (`BedBes` equivalent) — the legacy's `BedBes` alone used
  `<= D1`, splitting the opening period one day differently from every other ledger
  (`11-open-decisions.md` B5). One boundary rule, applied uniformly, closes this permanently.
- Running balance: credit-positive (`Σ(credit − debit)` up to and including the current row),
  matching the legacy's sign convention (`04-03-a.md` §3.1.b) — keep it, since Phase 3.2's party
  balance already committed to the same convention; don't introduce a second sign convention.
  **Fix the ordering tie-break**: order by `(date, voucher_number, line_id)` — the legacy had no
  tie-break below the voucher number, so two lines of the same voucher against the same account got
  an unspecified, non-reproducible order (`04-03-a.md` §3.1.c). Return `{ amount, side: Debit|Credit }`
  as one structured value, not the legacy's parallel grid-vs-print sign/letter duplication.
- Permission check: `Is_Admin_Or_Valid_Daftar`'s real rule is "no segment of the account is locked,"
  not "admin only" — the legacy's own refusal message was wrong about its own rule (`04-03-a.md`
  §3.1's permission-gate note). Implement the real rule; don't reproduce the misleading message.
- Cross-fiscal-year view: preserve the "all fiscal periods" option as an explicit, typed
  `fiscal_year_id: Option<...>` parameter (per `04-03-a.md` §3.1's note on `CO_ID = 0`), not a magic
  sentinel value threaded through string-built SQL.

**Spec refs:** `04-03-a/b/c-general-and-subsidiary-ledgers.md`; B4, B5, B6 in `11-open-decisions.md`.

**Manual test**

1. Post several vouchers → open the general ledger → confirm it shows data immediately (direct B6
   test).
2. Construct a case with both debit and credit activity before the report's from-date → confirm the
   opening balance is a single correct net figure, not confusing gross totals.
3. Reproduce a scenario that would trigger B4 (opening leg omitting a kind) and B5 (one-day boundary
   mismatch) in the aggregate/party ledgers → confirm both use the identical, correct predicate.
4. Post two lines of the same voucher against the same account → confirm their order in the ledger
   is stable and reproducible across repeated queries (direct tie-break fix test).
5. Lock one segment of an account, attempt to view its ledger as a non-admin → rejected with a
   message describing the actual rule (lock, not admin-only).

**Done when:** the general ledger shows data with no separate build step required, and B4/B5's
specific numeric discrepancies cannot be reproduced.

---

## 6.3 Card Jari statement

**Goal:** the party running-account statement — reads the balance logic already built in Phase 3.2.

**Build**

- Render the per-control-account breakdown and running balance for a chosen party and fiscal year,
  reusing 3.2's `Jari_Rem`-equivalent function directly — no separate reimplementation.
- Drill-through into the subsidiary ledger (6.2) for any control account in the breakdown.

**Spec refs:** `04-04-a/b-card-jari.md`.

**Manual test**

1. Open the Card Jari view for the party from Phase 3.2's worked-example test → confirm the total
   matches exactly (−21,000,000, then −16,000,000 after the Tafsil-2 addition).
2. Drill into one of the control accounts → lands on the correct subsidiary ledger, correctly scoped.

**Done when:** Card Jari and the Phase 3.2 API agree exactly, with no duplicated balance logic.

---

## 6.4 Stock / warehouse reports

**Goal:** warehouse activity and pistachio-operations reports, with the destructive-update and
cross-year-leak defects removed (B3, B16, B25).

**Build**

- Warehouse in/out and stock-balance reports, using the **single canonical stock formula** from
  Phase 5.3 — not a fourth reimplementation. Every report in this domain reads the same function; no
  report gets its own copy of the on-hand math.
- **B3 fix**: no report performs a write. The legacy's `Anbar_Amalkard` ran
  `UPDATE Anbar_FactorD SET AFD_Customer = (…)` with no `WHERE` clause on every single run
  (`00-overview.md` fact 5). Reports are read-only, full stop — this is enforced structurally by
  using a read-only database role for report queries if practical, and at minimum by code review
  discipline: a report handler that contains an `UPDATE`/`INSERT`/`DELETE` is a defect by definition
  in this phase.
- **B16 fix**: no report creates or drops permanent tables at runtime. The legacy's `RoyatJU` dropped
  and recreated a permanent table (`temp_RJ_<userId>`) on every run (`00-overview.md` fact 5) —
  operationally risky (concurrent users, orphaned tables on crash). Use a real temp table/CTE scoped
  to the query, or compute in application code; never `CREATE`/`DROP` a permanent object as part of
  serving a report.
- **B25 fix**: every report that filters by date range **also** filters by fiscal year explicitly —
  the legacy's `Anbar_ReportKharidForoosh` (purchase/sales report) had "no fiscal-year parameter and
  no `AFD_Coid` predicate anywhere in its body," so a date range spanning more than one fiscal year
  silently pulled rows from every year that fell in it (`11-open-decisions.md` B25). Add an explicit
  fiscal-year predicate to this report and audit every other date-range report in this phase for the
  same gap while building it.
- Pistachio operations reports (the three-variant `AnbarReportU` equivalent): read `04-01-a/b/c.md`'s
  catalogue entries for exact scope before implementing.

**Spec refs:** `04-01-a/b/c-report-catalogue.md`; B3, B16, B25 in `11-open-decisions.md`.

**Manual test**

1. Run the warehouse in/out report against a large dataset, then re-check the underlying movement
   table's data — confirm nothing changed (direct B3 regression test; the legacy would have silently
   mutated `AFD_Customer` on every run).
2. Run the same report concurrently from two sessions (or in quick succession) → confirm no runtime
   table creation/drop occurs and both runs succeed cleanly (direct B16 test).
3. Run the purchase/sales report with a date range spanning two fiscal years → confirm it either
   requires an explicit fiscal-year selection or correctly scopes to one year, and does **not** return
   rows from both years pooled together (direct B25 test).
4. Confirm the stock-balance figures in this report exactly match Phase 5.3's canonical on-hand query
   for the same item/date — no divergence between the two.

**Done when:** B3, B16 and B25 each have a manual test that would have failed against the legacy and
passes here, and every stock figure traces to the one canonical formula from Phase 5.3.

---

## 6.5 Print pipeline

**Goal:** server-side, RTL-correct document rendering — replacing FastReport, which the legacy used
for every printable surface with no PDF output at all (`04-06-a.md` §6's opening line: "no `TPrinter`
drawing, no HTML, no PDF library").

**Build**

- Server-side PDF generation for every report/document in Phases 2–6 (vouchers, invoices, ledgers,
  trial balances, cheque/petty-cash documents) — per `10-target-architecture.md` §3.4, browser print
  CSS is not a reliable substrate for tax-purpose documents that must be reproducible byte-for-byte.
- Header/letterhead/footer content, structured once, not injected ad hoc per report: organisation
  name and logo (from Phase 1's `organization` table, including the uploaded logo asset — the legacy
  had exactly one letterhead-image mechanism, worth keeping as a single reusable template concept per
  `04-06-a.md` §6.4), fiscal-year caption, report title, period range, page numbering, amount-in-words
  (Persian), signature block text (from `app_settings`, Phase 1). Build **one** structured "document
  header" object every print template consumes — not the legacy's 15+ string-literal memo names
  matched by convention with no compile-time safety (`04-06-a.md` §6.1's "data binding is by
  convention, not by contract" warning, and its note that renaming silently breaks the injection).
- Amount-in-Persian-words: reimplement `Str2String`'s output faithfully (thousand/million/billion
  scale words) as a proper function taking a numeric amount, not a string sliced with `Copy` — fix
  the legacy's string-input fragility (`04-06-a.md` §6.4's note that a thousands separator in the
  input "produces garbage") by never taking a pre-formatted string as input in the first place.
- **B23 fix**: no end user can edit or load a report layout. The legacy's FastReport preview exposed
  `pbEdit`/`pbLoad` on every single report to every user who could open a preview
  (`04-06-a.md` §6.2's explicit callout: "any user who can open any preview can edit the report
  layout... That is a live capability, not a theoretical one"). There is no equivalent surface in the
  rebuild at all — templates are server-side code/assets, not end-user-editable objects.

**Spec refs:** `04-06-a/b-print-pipeline.md`; `10-target-architecture.md` §3.4; B23 in
`11-open-decisions.md`.

**Manual test**

1. Generate a PDF for a posted voucher → confirm letterhead, organisation name, fiscal-year caption,
   amount-in-words, and signature block all render correctly from the structured header, not
   individually-injected strings.
2. Generate a PDF for a report with a numeric amount (e.g. a trial balance total) → confirm the
   Persian amount-in-words is correct for a large figure (billions) and for an edge case (a number
   with an internal zero group).
3. Confirm there is no UI control anywhere that lets a normal user modify a report's layout or load a
   different template (direct B23 test — the legacy exposed this on every single preview).
4. Confirm the same PDF, regenerated twice for the same data, is byte-identical (or effectively so) —
   this is the "reproducible for tax purposes" requirement from the target architecture doc.

**Done when:** every document type has a working PDF export with correctly structured, non-editable
templates, and B23's "any user can edit any layout" hole cannot be reproduced.

---

## 6.6 Export pipeline

**Goal:** clean CSV/Excel exports — replacing the legacy's "export the rendered page" model — with
the tax-authority export fixed (B17, and the column-header mismatch it exposed).

**Build**

- Per-report export endpoints returning clean tabular `text/csv` (UTF-8, RFC 4180 quoting — not the
  legacy's ANSI/Windows-1256 CSV with unquoted commas that "breaks the row" on any description
  containing one, `04-07.md` §7.1's specific warnings) or `.xlsx` via `rust_xlsxwriter`
  (`10-target-architecture.md` §3.4). Do **not** port "export the rendered page" — no merged cells, no
  repeated page headers, no blank spacer rows; each export is a clean data table, re-importable.
- **B17 fix — tax-authority export.** Filter to posted (`status = 2`) vouchers only — the legacy's
  `ToExcelDaraeiU` export had no `M_Tx` filter at all, so "draft vouchers are exported to the tax
  authority," which `04-07.md` §7.2 calls "the most consequential instance of the missing state
  filter in the whole system." This is a direct instance of the same rule already established in
  6.1/6.2 (posted-only for anything leaving the system) — apply it here without exception, since this
  export leaves the building.
- **Also fix the column-header defect found alongside B17**: the legacy's export's first column was
  headed "row number" (`ردیف`) but actually contained the voucher number — "a real defect in a file
  submitted to the tax authority" (`04-07.md` §7.2's closing note). Label every column by what it
  actually contains; verify the final column order against whatever the tax authority's current
  template expects before shipping this export for real use — treat that verification as a
  precondition, not an assumption.
- Text cleaning for exported narration: collapse whitespace runs properly (not the legacy's single
  double-space-to-single-space pass that misses runs of three-plus spaces), and if truncation to a
  fixed width is still required by the target format, apply it in a way that doesn't cut mid-word
  where avoidable.
- No image/JPEG export of rendered pages — not a real requirement once PDF (6.5) and clean data
  exports exist side by side; don't build the legacy's incidental raster-export capability.

**Spec refs:** `04-07-export-pipeline.md`; B17 in `11-open-decisions.md`.

**Manual test**

1. Export the tax-authority voucher report for a range including both draft and posted vouchers →
   confirm only posted vouchers appear in the output (direct B17 test).
2. Confirm the exported file's column headers accurately describe their contents — specifically that
   whatever column holds the voucher number is labelled as such, not "row number."
3. Export a report containing a narration with an embedded comma and with multiple consecutive spaces
   → confirm the CSV parses correctly in a standard tool (comma doesn't break row structure) and the
   whitespace is cleanly collapsed.
4. Open an exported `.xlsx` in a spreadsheet tool → confirm it's a clean data table (no merged cells,
   no repeated headers, no blank spacer rows).

**Done when:** the tax-authority export contains only posted vouchers with correctly labelled
columns, and every export is a clean, re-importable data table.

---

## 6.7 Report permission gating

**Goal:** every report route checked server-side — closing the legacy's ungated menu items (B24) and
tightening the inconsistent gating on party-card reports.

**Build**

- Map every report to its permission id from the seeded catalogue (Phase 1.1/2.8's pattern, extended
  to the report ids: 1122–1124, 1128, 1131, 1141, plus the ones the legacy left ungated).
- **B24 fix**: the legacy left five report menu items with **no permission check at all**
  (`Report4`/`5`/`6`/`8`, `_Report9`) and had an inconsistency where `CardJariU.Report2` was ungated
  while `Report1` on the same screen was gated (`11-open-decisions.md` B24). Every report route in
  the rebuild requires an explicit, real permission check — there is no "forgot to gate this one"
  category possible if every route is enumerated against the seeded catalogue as a checklist (same
  approach as Phase 2.8) and none are left off by omission.
- Re-verify B23 (6.5) is still closed once every report is wired through this permission layer — the
  two defects (ungated menu items, editable layouts) are independent and both must hold simultaneously.

**Spec refs:** `04-01-a/b/c-report-catalogue.md` (for the full report list); `08-04-authorization.md`
§4.4; B23, B24 in `11-open-decisions.md`.

**Manual test**

1. Enumerate every report route built in this phase against the permission catalogue → confirm each
   one has an assigned permission id (no unmapped routes).
2. As a user without the relevant permission, attempt each report route directly (not just through
   the UI) → confirm every single one returns `403`, including the ones the legacy left ungated
   (direct B24 test — pick at least the equivalents of `Report4`/`5`/`6`/`8`).
3. Confirm a party-card report and its sibling report on the same screen are gated consistently (no
   repeat of the `Report1`-gated/`Report2`-ungated split).

**Done when:** a scripted sweep of every report route confirms 100% permission coverage — none
reachable without an explicit grant.
