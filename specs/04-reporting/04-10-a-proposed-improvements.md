_Part of [04-reporting](../04-reporting.md) — [index](00-index.md)_

## 10. PROPOSED IMPROVEMENTS (needs user approval)

> **Everything in this section is a suggestion, not a specification.** The default for the rebuild is
> **port as-is**: reproduce the legacy behaviour exactly, bugs included, so that old and new output
> can be reconciled row for row. Nothing below is implemented unless the user explicitly approves it,
> item by item. Items are grouped by whether they change **numbers**, change **behaviour**, or change
> only **presentation and safety**.
>
> Each item: current behaviour → proposed change → why → risk.

---

### Group A — changes that alter reported numbers

These need accountant sign-off, and each one should ship behind a flag with the legacy behaviour
reproducible for reconciliation.

#### A1. Make the voucher-state filter real everywhere

- **Current.** The 4-column trial balance's three state checkboxes (`اسناد در حال تحریر`,
  `اسناد تایید شده`, `اسناد ثبت دائم شده`) are decoration: they gate the button and are never used in
  the `WHERE` clause (§2.1(a)). Every ledger, Card Jari, the party balance list, the voucher summary
  and the tax-authority export include state-0 drafts unconditionally. The 6-column trial balance is
  the only report that filters, via `@Sabt`.
- **Proposed.** One `states: Set<VoucherState>` parameter on every report endpoint, surfaced in the
  UI, defaulted per report to the legacy behaviour, and **echoed in the report header** ("includes
  drafts" / "posted only").
- **Why.** The two trial balances currently cannot be reconciled by design (§2.3). Users have no way
  to know whether a number includes drafts. The controls promise a filter that does not exist.
- **Risk.** **High.** Every historical figure changes if the default is changed. Mitigate by
  defaulting to legacy behaviour and making the filter opt-in.

#### A2. Exclude drafts from the tax-authority export by default

- **Current.** `ToExcelDaraeiU` exports every line in the date range with no `M_Tx` filter (§7.2) —
  unposted drafts are submitted to the tax authority.
- **Proposed.** Default `states = {Posted}`, with an explicit override that warns.
- **Why.** Submitting provisional entries as final records is a compliance exposure, not a
  preference.
- **Risk.** Medium. Row counts and totals in the export will drop. Must be validated against Q9
  (§9.2) before changing.

#### A3. Unify the opening-balance boundary

- **Current.** Ledgers split at `entry_date < from_date`; the party balance list splits at
  `entry_date <= from_date` with turnover `> from_date` (§1.2, §5.3). Entries dated exactly on the
  from-date land on opposite sides.
- **Proposed.** One rule everywhere: opening = `< from_date`, period = `>= from_date and <= to_date`
  (the ledger convention, which is the standard one).
- **Why.** The two reports are supposed to agree for a party and silently do not.
- **Risk.** Medium. Changes `Rem1`, `GBed`, `GBes` on R09 for any party with activity on the
  from-date. Needs Q8 answered first.

#### A4. Fix the `TMoein` opening-balance double count

- **Current.** The consolidated ledger's opening leg omits `M_kind = 1` while its movement leg
  includes it (§3.3(b)), so the opening balance also sums journal-summary rows.
- **Proposed.** Add `M_kind = 1` to the opening leg.
- **Why.** It is unambiguously a bug: the two legs of the same report disagree about what data they
  are reading, and the running balance is wrong by a constant for every account with prior
  journal-summary activity.
- **Risk.** Low as a correctness matter, **medium** as a reconciliation matter — every consolidated
  ledger opened from Card Jari changes.

#### A5. Add `M_kind` filtering to `Dm.Jari_Rem` and `BedBes`

- **Current.** Neither filters `M_kind` (§4.3, §1.2), so journal-summary rows can be included in a
  party's balance.
- **Proposed.** Add `M_kind = 1`.
- **Why.** Same reason as A4. Affects only accounts reachable by a `SahamdarConfig` template with
  `SC_M = 0`, so the blast radius depends on Q11.
- **Risk.** Low, contingent on Q11.

#### A6. Add a `GROUP BY` to `Dm.Jari_Rem`

- **Current.** `QList` deduplicates `SahamdarConfig` templates with `GROUP BY SC_K, SC_M, SC_T`;
  `Jari_Rem` does not (§4.3), so duplicate templates double-count into the "final balance".
- **Proposed.** Mirror `QList`'s `GROUP BY`.
- **Why.** Two queries over the same configuration table must expand it the same way.
- **Risk.** Low, contingent on Q11 finding duplicates. If there are none, this is a no-op safety net.

#### A7. Add a mandatory balance assertion to the trial balances

- **Current.** Neither trial balance checks that debits equal credits, and neither can display a
  difference — a discrepancy is silently absorbed into the two clamped columns (§2.1).
- **Proposed.** Compute `Σ debit − Σ credit` at Kol level, display it, and flag the report as
  out of balance when it is non-zero.
- **Why.** A trial balance whose whole purpose is to detect imbalance cannot detect imbalance.
- **Risk.** Low numerically (nothing changes unless the books are already broken) but **potentially
  alarming operationally** — see Q4. Run Q4 before shipping this.

#### A8. Derive `RooznamehViewU`'s totals from lines, not from the `DMoein` cache

- **Current.** The journal voucher list displays `DM_TBed`/`DM_TBes` from the header cache, and the
  voucher print spells out `DM_TBed` in Persian words (§1.7, §6.4).
- **Proposed.** Compute from `voucher_lines`.
- **Why.** The cache is drift-prone by established fact; this is the one screen that presents it as
  authority, and the amount-in-words on a printed voucher is the most consequential place for it to
  be wrong.
- **Risk.** Low, but Q5 will show how many vouchers currently display a different number.

#### A9. Reconcile or retire one of the two trial balances

- **Current.** Two trial balances share no code, no column names and no definition of "turnover"
  (§2.3).
- **Proposed.** One engine with an `as_of` / `from..to` mode switch, rendering both legacy layouts.
- **Why.** §2.3's whole point. Also the only way to satisfy A1 and A7 once rather than twice.
- **Risk.** Medium — depends entirely on Q1 (the `Taraz_6Sotooni` body). Do not attempt before that
  is dumped.

---

### Group B — changes to behaviour that do not alter numbers

#### B1. Add a deterministic third sort key to every listing

- **Current.** `ORDER BY M_Date, M_Sanad` with no tie-break; running balances between lines of the
  same voucher are not reproducible run to run (§3.1.c).
- **Proposed.** `(entry_date, voucher_no, line_id)`.
- **Why.** Reproducibility is a precondition for reconciliation, snapshot tests and audit.
- **Risk.** Low. Row *order within a voucher* may differ from any given legacy printout; totals do
  not change.

#### B2. Delete the per-user result tables

- **Current.** `RoyatJU` creates, drops and repopulates a permanent table `temp_RJ_<userId>` in the
  application database on every run (§1.4); `Report7U` would do the same if it were reachable.
- **Proposed.** CTEs; if caching is needed, cache in the API layer keyed by filter hash.
- **Why.** Requires DDL rights for every reporting user, leaks one table per user forever, and makes
  concurrent runs by the same user race.
- **Risk.** Low. Performance must be measured — the legacy design exists because the roll-up is
  expensive.

#### B3. Validate account selection before running a ledger

- **Current.** `DKolU` runs with an empty account code and produces `And M_Ko=` → a SQL syntax
  error dialog; its `F_Valid` flag is written and never read (§3.1.i).
- **Proposed.** Require an account; disable the run button until one is selected.
- **Risk.** None.

#### B4. Fix the `Taraz6SetooniU` validation holes

- **Current.** `D2` is never validated (line 61 re-tests `d1`); the `D1 > D2` check sets focus but
  does not `Exit`, so an inverted range runs and returns an empty period (§2.2).
- **Proposed.** Validate `D2`; `Exit` on an inverted range.
- **Risk.** None.

#### B5. Replace `Farsi_day := 31` defaults with a real month end

- **Current.** Nine screens default the to-date to day 31, which does not exist in seven Jalali
  months (§5.4). Harmless as a string bound; invalid as a date.
- **Proposed.** Default to the actual last day of the month, or model the bound as exclusive on the
  first of the next month.
- **Why.** The moment anything parses the value through a real calendar it breaks — including the
  rebuilt backend.
- **Risk.** Low. Results are identical; only the displayed default changes.

#### B6. Warn (or clamp) when a report date falls outside the fiscal year

- **Current.** No reporting screen checks `D1`/`D2` against `Base.FromDate`/`ToDate`, even though
  every data-entry screen does (§5.4). An out-of-range date returns silently empty.
- **Proposed.** A non-blocking warning; optionally clamp, as `Anbar_MandehU` already does.
- **Risk.** Low.

#### B7. Make double-click consistent

- **Current.** Double-click opens the voucher in `DKolU`, `DMoein`, `TMoein`; opens the party summary
  in `BedBes`; descends a level in `DaftarT_U` and `RoyatJU`; and **toggles row selection** in
  `CardJariU` (§4.9).
- **Proposed.** Double-click always drills in; selection is by checkbox.
- **Risk.** Low, user-visible.

#### B8. Reload rather than clear when the fiscal year changes

- **Current.** Three different behaviours: the ledgers reset the dates and close the result set; Card
  Jari clears the screen and requires re-focusing `S_Card`; `BedBes` closes the result set (§5.6).
- **Proposed.** Re-run the report with the new year.
- **Risk.** Low; slightly more database traffic.

#### B9. Sort the party balance list by amount

- **Current.** `order by jari` — by card number (§1.2).
- **Proposed.** Default to closing balance descending, with column sorting.
- **Why.** It is a "who owes us most" report that cannot answer that question.
- **Risk.** None.

#### B10. Drop the hard-coded amount window on the party balance list

- **Current.** Defaults `M1 = 1 000 000`, `M2 = 100 000 000`, silently excluding balances outside
  that band (§1.2).
- **Proposed.** Empty by default.
- **Why.** A filter nobody set is hiding rows.
- **Risk.** Low, but row counts jump on first use.

#### B11. Close the permission gaps

- **Current.** `Report4`, `Report5`, `Report6`, `Report8` and `_Report9` have no `IsEnabel` call
  (§1.0); `CardJariU.Report2` is ungated while `Report1` is gated by key 1123 (§4.7); the per-account
  lock check in Card Jari runs *after* the identity data and photo are displayed (§4.7); the refusal
  messages claim "administrators only" when the rule is a per-account/per-party lock flag (§3.1).
- **Proposed.** Named permissions on every report; check before loading any data; correct the
  messages (§11.5).
- **Risk.** Low technically, **user-visible**: some users will lose access they currently have by
  omission.

#### B12. Remove designer access from the report preview

- **Current.** `PreviewOptions.Buttons` includes `pbEdit` and `pbLoad` on all 58 reports (§6.2) — any
  user can redesign a report or load a foreign `.fr3`.
- **Proposed.** Report templates are server-side and versioned; users get print, export and zoom.
- **Risk.** Low, unless someone has been relying on it — see Q20.

#### B13. Make voucher-template selection a setting

- **Current.** Which of `PrintNu` / `PrintM2U` / `PrintMU` runs is decided by which line is commented
  out in `SanadEditU.pas:521-529` (§6.5).
- **Proposed.** A `print_settings.voucher_template` value.
- **Risk.** None.

---


---

[← SS9 Open questions](04-09-open-questions.md) | [Index](00-index.md) | [SS10 PROPOSED IMPROVEMENTS (2/2) →](04-10-b-proposed-improvements.md)
