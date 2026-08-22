_Part of [04-reporting](../04-reporting.md) — [index](00-index.md)_

### 3.5 وضعیت حسابهای کل — monthly turnover of a Kol account (`KolStateU`)

Listed here because it is the fifth "ledger-ish" screen. **Launched from** `Mainu.pas:647-650`
(`TMain.Report4Click` → `KolState.init`), menu item `Report4` — but the menu caption is
`لیست کنترلی` ("control list", `Mainu.dfm:10745-10747`), which matches neither the form caption
`وضعیت حسابهای کل` (`KolStateU.dfm:5`) nor the report title. Reachable.

Not to be confused with the dead `KolSatateU.pas` (note the transposed letters) — that file contains
`procesdure` and has never compiled.

- **Parameters:** `Kol: TEditInt` (`کد کل`, `.dfm:31`) and `L_Kol: TDBLookupComboBox`, kept in sync in
  both directions (`:76-92`). `SabtClick` refuses unless the typed code equals the picked key
  (`:97-101`) — a silent `Exit`, no message.
- **Lookup source:** stored procedure `Select_Kol;1` (`.dfm:88`), no parameters beyond
  `@RETURN_VALUE`.
- **Data source:** stored procedure **`KolState;1`** (`.dfm:105`), parameters `@kol` (`.dfm:115`) and
  `@CoID` (`.dfm:122`), set at `KolStateU.pas:103-104` from `Dm.CO_ID` and the typed code. **Body not
  in this repo** — see §9. No date range at all: the whole fiscal year.
- **Output columns consumed** (`.dfm:326-407`): `Sal` (year), `mahstr` (month name string), `M_Bed`,
  `M_Bes`. Report headers, RTL order: `ردیف` (`[line#]`), `سال`, `ماه`, `گردش بدهکار`,
  `گردش بستانکار` (`.dfm:195,213,231,249,267`). `'%2.0n'` numeric format on the two amounts. A4
  portrait (`.dfm:170`). **No footer totals band at all** — the report ends after `MasterData`.
- **Print-time injection** (`:106-107`): `Reg` ← `Dm.RegName`, `kol` ←
  `'وضعیت گردش حساب ' + kol.Inttext + ' ' + L_Kol.Text`. Both memos carry the stale design-time text
  `گردش بدهکار` (`.dfm:284,301`). Note this unit uses `Rp1.FindComponent` rather than the
  `Rp1.FindObject` used everywhere else (`:106-107` vs e.g. `DKolU.pas:185`); if a future FastReport
  version stops parenting memos directly to the report this silently returns `nil` and raises on the
  cast.
- There is **no on-screen grid**: `SabtClick` goes straight to `rp1.ShowReport(true)`.
- **Writes:** none from Delphi; unknown inside `KolState;1`.

---

### 3.6 Cross-cutting summary for the rebuild

| Question | Answer |
|---|---|
| Opening balance rule | `M_Date < from_date`, same account/kind/year, **two gross sums, never netted** |
| Opening row identity | synthetic: `RN=0`, `date=from_date`, `voucher=0`, `شرح='مانده از قبل '` |
| Opening row suppression | dropped when both sums are zero/NULL |
| Running balance | `Σ(credit − debit)` inclusive, **credit-positive**, correlated subquery, O(n²) |
| Running-balance display | grid: signed; print: `ABS()` + `بس`/`بد` letter |
| Ordering | `(M_Date ASC, M_Sanad ASC)` — **no third tie-break**, non-deterministic within a voucher |
| Date comparison | string comparison on `'yyyy/mm/dd'` (§5) |
| Voucher states | **all four ledgers include state-0 drafts**; `M_Tx` is displayed, never filtered |
| Source of truth | `Moein` lines in all cases; the `DMoein` header cache is never read by any ledger |
| Fiscal-year scope | `DKolU`/`DMoein`/`TMoein`: picker incl. synthetic "all periods" (`CO_ID = 0`); `DaftarT_U`/`KolStateU`: `DM.CO_ID` only |
| Writes | none in any of the five units |


---

[← SS3 General and subsidiary ledgers (2/3)](04-03-b-general-and-subsidiary-ledgers.md) | [Index](00-index.md) | [SS4 Card Jari (1/2) →](04-04-a-card-jari.md)
