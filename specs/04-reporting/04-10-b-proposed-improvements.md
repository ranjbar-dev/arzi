_Part of [04-reporting](../04-reporting.md) — [index](00-index.md)_

### Group C — presentation, safety and hygiene

#### C1. Produce real digits, not font-substituted ones

- **Current.** Persian numerals exist only as glyphs of the `WeblogmaYekan` font, in one report
  (§6.7). Copy, export and search all yield Latin digits; two non-standard fonts must be installed.
- **Proposed.** `Intl.NumberFormat` with an explicit `numeralSystem` preference, applied
  system-wide.
- **Risk.** Low. Exported files change (for the better).

#### C2. Emit UTF-8, RFC 4180 CSV

- **Current.** `CSV.UTF8 = False`, `OEMCodepage = False`, `ForcedQuotes = False`, `Separator = ','`
  (§7.1) — ANSI (Windows-1256) output that breaks on any comma in a description.
- **Proposed.** UTF-8 with BOM, quoted fields.
- **Risk.** None, unless a downstream consumer expects 1256.

#### C3. Export data, not rendered pages

- **Current.** All three FastReport filters run with `DataOnly = False`, so exports carry letterhead,
  repeated headers, page breaks and blank spacer rows (§7.1); `MoeinZipU`'s Excel export writes every
  cell as text and never saves the file (§7.3).
- **Proposed.** Per-report `.csv` / `.xlsx` endpoints with typed cells, plus a separate PDF of the
  rendered layout.
- **Risk.** Low; the exported layout changes visibly.

#### C4. Add PDF output

- **Current.** No PDF export exists anywhere in the project (§7.5).
- **Proposed.** Server-side PDF per report.
- **Risk.** None.

#### C5. Remove the hard-coded connection strings from source

- **Current.** Design-time `ConnectionString` properties with `User ID=sa` and named catalogues/hosts
  (`Arzi89` / `MOHSEN-RANJBAR\SQLEXPRESS`, `RPPC` / `PESTEH`) sit in `DKolU.dfm:517-522,1185-1190`,
  `CardJariU.dfm:6294-6304,6487-6492,6532-6537`, `BedBes.dfm:1108-1113`, `Dmu.dfm:8642-8647` and many
  others. They are overwritten at runtime but are in source control.
- **Proposed.** Configuration only; nothing in the repository.
- **Risk.** None. See `08-platform-and-security.md`.

#### C6. Wrap the Excel automation in `try…finally`

- **Current.** `ToExcelDaraeiU:113-178`, `MoeinZipU:392-427` and `Anbar_Amalkard` create an Excel COM
  object with no protection; any exception leaves an invisible `EXCEL.EXE` holding the workbook, with
  `displayAlerts` still `False` (§7.2, §7.3).
- **Proposed.** Moot in the rebuild — generate `.xlsx` server-side with no Office dependency.
- **Risk.** None.

#### C7. Fix the tax-export column header

- **Current.** Column A is headed `ردیف` ("row number") and contains `M_Sanad` (the voucher number),
  §7.2.
- **Proposed.** Head it `شماره سند`, or emit a genuine row number and add the voucher number as its
  own column.
- **Why.** A mislabelled column in a statutory file.
- **Risk.** **Must not be changed unilaterally** — the authority's template may expect exactly this.
  Blocked on Q9.

#### C8. Fix the INI persistence bug in the 4-column trial balance

- **Current.** `Taraz4Setooni_U.pas:191-192` writes key `'TX0'` on the false branch for both `R1` and
  `R2`, so unticking either clears `R0`'s saved state (§2.1(a)).
- **Proposed.** Moot if A1 is approved (the controls become real and persist properly); otherwise
  delete the controls.
- **Risk.** None.

#### C9. Make the fiscal-year picker on the 4-column trial balance do something

- **Current.** `COID.KeyValue` feeds only `Dbo.Make_R`'s `@Co` argument — it changes how account
  codes are *formatted*, never which rows are read (`DM.CO_ID` does that). It also initialises to the
  newest year, which need not be the selected one (§2.1(b)).
- **Proposed.** Either filter by it or remove it.
- **Risk.** Medium if "filter by it" is chosen — the report starts returning different data for users
  who have been changing the dropdown.

#### C10. Fix the mislabelled and inconsistent UI strings

- **Current.** `B_Close` in Card Jari is hinted `چاپ` ("print") and closes the form (§4.8); the
  `KolStateU` menu item is captioned `لیست کنترلی` ("control list") for a form titled
  `وضعیت حسابهای کل` (§3.5); `DaftarT_U`'s printed level titles are off by one (§3.4); two different
  Persian wordings exist for "select at least one item" and for "invalid date range" (§11.5); the
  "administrator only" messages describe the wrong rule (§3.1, §4.7); `تو سط` is a typo for `توسط`;
  the accounting-side "not found" message is written into the shareholder-side name box (§4.6).
- **Proposed.** One locale file, reviewed once.
- **Risk.** None.

#### C11. Delete the dead code rather than porting it

- **Current.** §1.13 lists 14 dead units and report objects plus a dozen dead controls inside live
  screens.
- **Proposed.** Do not port. Confirm with the customer first — several are dead only because one line
  is commented out.
- **Risk.** Low, but confirm.

#### C12. Replace the O(n²) running balance

- **Current.** `(Select Sum(M_Bes-M_Bed) from #R as N Where N.RN <= #R.RN)` — a correlated subquery
  per row, in a statement that already uses `ROW_NUMBER()` (§3.1.b).
- **Proposed.** A window function.
- **Why.** Purely performance; results are identical.
- **Risk.** None.

#### C13. Replace the N+1 query pattern in Card Jari

- **Current.** `2 × <accounts> + 3` serial round trips per party (§4.4).
- **Proposed.** One query.
- **Risk.** None.

#### C14. Remove the trailing blank row in Card Jari

- **Current.** `Vt1.Append` at `CardJariU.pas:163` leaves the grid in insert mode, showing a blank
  selectable row that produces a SQL syntax error if selected before pressing `مشاهده تجمیعی`
  (§4.4, §4.9).
- **Proposed.** Moot in the rebuild.
- **Risk.** None.

#### C15. Initialise `J` in the voucher-gap loop

- **Current.** `Report6U.pas:98-104` — `J` is read by `inc(J)` and `J mod 20` without being
  initialised (§1.3).
- **Proposed.** Moot in the rebuild.
- **Risk.** None.

---

### Recommended sequencing if the user approves

1. **First, before any code:** Q1, Q2, Q3, Q4, Q5 from §9 — the artefacts. Then Q6, Q7, Q8, Q9 —
   the accountant decisions.
2. **Group C** wholesale: none of it changes a number, and most of it disappears simply by rebuilding.
3. **Group B** with B1 (deterministic ordering) first, because reconciliation depends on it.
4. **Group A** last, one item at a time, each behind a flag, each reconciled against the legacy
   output before the flag is flipped. A7 (the balance assertion) should be shipped in warn-only mode
   first.


---

[← SS10 PROPOSED IMPROVEMENTS (1/2)](04-10-a-proposed-improvements.md) | [Index](00-index.md) | [SS11 Naming map →](04-11-naming-map.md)
