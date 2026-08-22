_Part of [04-reporting](../04-reporting.md) — [index](00-index.md)_

## 5. Date-range and fiscal-year filtering semantics

Every report in the system filters on exactly two axes: a **fiscal-year stamp** (`*_COID`) and a
**Jalali date string**. Both are simpler and more fragile than they look.

### 5.1 The stored date format — what can and cannot be verified here

**What is verifiable from this repository:**

- Every date column consumed by a report is compared as a **string** with `>=`, `<=`, `<` — never with
  a SQL date type, never through `CONVERT`, never with a `DATEDIFF`. There is not one date-typed
  comparison in any reporting unit.
- The value on the Delphi side always comes from `TFullDate.Farsi_Date` (or `TEditDate.Text`), which
  serialises into the `.dfm` as a zero-padded 10-character `yyyy/mm/dd`:
  `Farsi_Date = '1403/08/09'` alongside `Farsi_year = 1403`, `Farsi_month = 8`, `Farsi_day = 9`
  (`DKolU.dfm:139-143`). The design-time default of the 6-column trial balance's stored-procedure
  parameter is `@D1 varchar(10) = '1397/09/01'` (`Dmu.dfm`, quoted in §2.2), independently confirming
  the same shape and width from the database side.
- `TDm.MiladiToShamsi` (`Dmu.pas:362-437`) — the one Jalali conversion whose source *is* in the repo —
  emits exactly that shape, zero-padding month and day explicitly (`:430-434`).

**What is NOT verifiable from this repository, and must not be assumed:**

- **The column type.** `Moein.M_Date` may be `varchar(10)`, `char(10)`, `nvarchar(10)` or something
  wider. String comparison behaves differently across these (`char` pads, `varchar` does not; a wider
  column silently admits malformed values). Dump the schema.
- **The collation.** All range filtering is ordinal-comparison-dependent. A non-binary,
  accent-insensitive or Arabic collation changes nothing for ASCII digits and `/`, but it would matter
  the moment a Persian-digit value (`۱۴۰۳/۰۸/۰۹`) got written. Whether any exist is unknown.
- **Whether every stored value is well-formed.** Nothing in the application normalises on write, and
  several write paths take `.Text` rather than `.Farsi_Date` (see §5.5).
- **The `TFullDate` conversion algorithm itself.** `Tools.pas` is not in the repository — only the
  compiled unit is referenced (`uses … Tools` in `DKolU.pas:10`, `TMoein.pas:9`, `KolStateU.pas:7`,
  and ~40 other units). `Farsi_Date`, `Farsi_Valid`, `Farsi_day`, `Farsi_year`, `Farsi_month` and
  `SetToDate` are the only members the application uses (252, 39, 14, 4, 2 and 40 occurrences
  respectively across all `.pas`). **Its leap-year rule, its epoch offset and its validation rule
  cannot be determined from source.** Every statement below about *ordering* holds regardless; every
  statement about *which Gregorian instant a Jalali date denotes* does not.

`TDm.MiladiToShamsi` is a **second, independent implementation** of the same conversion living on the
data module (`Dmu.pas:140` declaration, `:362-437` body). Grep finds **no call site anywhere** — it is
dead code. It is worth reading anyway because it documents the intent: month lengths 31×6 then 30×6
(`:407-427`) and a leap rule of `(Year - 1) mod 4 = 0` applied to the *Gregorian* year (`:388`). If
`Tools.TFullDate` uses a different (e.g. 33-year cycle) rule, the two disagree on roughly one day in
128 years' worth of dates. Do not port either; use a vetted Jalali library and pin the rule in tests.

### 5.2 Why string comparison happens to work — and exactly where it stops working

`'yyyy/mm/dd'` with zero padding is **lexicographically order-isomorphic to chronological order**,
because every field is fixed-width and the separator `/` (0x2F) sorts below every digit (0x30-0x39).
So `M_Date >= '1403/01/01' and M_Date <= '1403/12/29'` is a correct range filter, and
`Order By M_Date` is a correct chronological sort. This is not an accident — it is the only reason the
system works at all — but it depends on four invariants that nothing enforces:

| Invariant | Enforced by | What breaks if violated |
|---|---|---|
| Always 4-digit year | `TFullDate` (unverifiable) | a 3-digit year sorts before everything |
| Always zero-padded month/day | `TFullDate` / `MiladiToShamsi:431-434` | `'1403/8/9' < '1403/10/01'` is **false** — month 8 sorts after month 10 |
| Always ASCII digits | nothing | Persian-Indic digits `۰۱۲…` sort above ASCII, so a single Persian-digit row sorts to the end of every ledger |
| Always the same separator | nothing | `-` (0x2D) sorts below `/`, silently reordering |

The second row is the realistic failure. `'1403/8/9'` is 8 characters, not 10, and any comparison
against a padded bound misbehaves. Nothing in the reporting code checks length.

**Recommendation for the rebuild:** store the date twice — a real `DATE` in Gregorian (the sortable,
indexable, arithmetic-capable truth) plus the Jalali `yyyy/mm/dd` string for display and for
byte-identical reproduction of legacy output. Do the conversion once, at the boundary, with a pinned
algorithm. Never range-filter on the string.

### 5.3 Boundary semantics — the shared model

Two boundary conventions exist and they are used consistently within each family:

**(a) Ledger / opening-balance model** (`DKolU`, `DMoein`, `TMoein`):

```
opening leg :  M_Date <  @D1          -- strictly less
movement leg:  M_Date >= @D1  and  M_Date <= @D2   -- inclusive both ends
```
(`DKolU.pas:277,290,291`; `DMoein.pas:650,665,666`; `TMoein.pas:238,250,251`)

The two legs **partition** the timeline at `@D1` with no gap and no overlap: an entry dated exactly
`@D1` is in the period, not the opening. **This is correct and there is no off-by-one.** It is the one
piece of date logic in the system that is unambiguously right, and it is right in all three units.

**(b) Flat-range model** (everything else): `>= @D1 and <= @D2`, inclusive both ends
(`DaftarT_U.pas:149-150`, `RoyatJU.pas:308`, `Report7U.pas:403,437`, `MoeinZipU.pas:258,590,600,649,672`,
`AnbarReportU.pas:210`).

**(c) As-of model:** `M_Date <= @D1` only, no lower bound — the 4-column trial balance
(`Taraz4Setooni_U.pas:103`, analysed in §2.1). Because `M_COID` is also applied, "no lower bound"
means "from the start of the fiscal year", which is why its `گردش` ("turnover") columns are really
cumulative columns.

### 5.4 Where the boundaries actually bite

**Default `Farsi_day := 31`.** Nine screens set the to-date's day to 31 unconditionally:
`Taraz6SetooniU.pas:134`, `Report7U.pas:335`, `Anbar_Amalkard.pas:156`, `Anbar_MandehU.pas:121`,
`AnbarReportKharidU.pas:97`, `ToExcelDaraeiU.pas:200`. Jalali months 7–11 have 30 days and Esfand has
29 or 30, so for seven months of the year this produces a **date that does not exist**. As a *string
upper bound* it is harmless and in fact convenient — `'1403/08/31' > '1403/08/30'`, so it means "the
whole month". But:

- It is displayed to the user as a real date, and users read it as one.
- It is stored into INI files and re-read on the next run.
- The moment anything parses it through a real calendar (an export, a stored procedure that does
  `CONVERT`, or the rebuilt Rust backend) it is rejected or silently shifted.
- The matching `Farsi_day := 1` on the from-date (`Taraz6SetooniU.pas:132`, `DaftarT_U.pas:315`,
  `BedBes.pas:159`, `Report7U.pas:334`, `Anbar_Amalkard.pas:155`, `Anbar_MandehU.pas:120`,
  `AnbarReportKharidU.pas:95`, `ToExcelDaraeiU.pas:198`) is always valid.

Port this as an **exclusive upper bound on the first day of the next month**, or as a
`month_end(year, month)` helper — not as "day 31".

**Inverted ranges.** Three different behaviours coexist:

| Unit | Check | On failure |
|---|---|---|
| `DKolU.pas:247-251` | `D1.Farsi_Date > D2.Farsi_Date` | focus `D1`, **`Exit`** — correct |
| `DMoein.pas:611-615` | same | focus `D1`, **`Exit`** — correct |
| `TMoein.pas:220-224` | same | focus `D1`, **`Exit`** — correct |
| `DaftarT_U.pas:132-137` | same | message `رنج تاریخ را به درستی وارد کنید`, focus, **`Exit`** — correct and the only one with a message |
| `Taraz6SetooniU.pas:66-69` | same | focus `D1`, **no `Exit`** — the query runs and returns an empty period (§2.2) |

**Validity checks.** `Farsi_Valid` is consulted 39 times. In the reporting surface the pattern is
`if D.Farsi_Valid = false then begin ActiveControl := D; Exit; end` — usually silent. Two exceptions:
`Taraz4Setooni_U.pas:82-87` shows `تاریخ را به درستی وارد کنید`; `DaftarT_U.pas:120-131` shows
`تاریخ را وارد کنید`. And one hole: `Taraz6SetooniU.pas:61` re-tests `d1` where it means `d2`, so the
6-column trial balance never validates its to-date (§2.2).

**Nothing checks the date against the fiscal year.** Not one reporting screen verifies that `D1`/`D2`
fall inside the selected year's `Base.FromDate`..`Base.ToDate`. Contrast the data-entry side, which
does it everywhere: `CheckDaryaftU.pas:248-253`, `CheckDaryaft2U.pas:166-171`, `CheckBargashtu.pas:187-192`,
`CheckEditU.pas:392`, `EnteghalU.pas:156-162,175-181` all refuse an out-of-range date with
`تاریخ باید در رنج <from> الی <to> باشد`. The single reporting-side exception is the inventory report
`Anbar_MandehU.pas:115-118`, which silently **clamps** `D1`/`D2` into the year rather than refusing.

Because the year is enforced by `M_COID` rather than by the dates, an out-of-range date is not
*incorrect* — it just returns nothing, or returns the whole year. But it means a user who types
`1402/05/01` while the app is on year 1403 gets a silently empty report with no explanation.

### 5.5 `.Farsi_Date` versus `.Text` — a real inconsistency

`Farsi_Date` is the parsed, normalised property; `Text` is the raw edit-box content. Reporting code
uses `Farsi_Date` almost everywhere, but not quite:

- `MoeinToRU.pas:164`: `' Where M_Date>='+QuotedStr(_D1.Text)+' and M_Date<='+ QuotedStr(_D2.Text)` —
  **raw text**, spliced into SQL that then drives a bulk `INSERT` (§1). If the control's `Text`
  carries a mask, a partially typed value or trailing whitespace, the filter silently mismatches.
- `AnbarCardJensiU.pas:60-61` and `EnteghalU.pas:319,327` assign `.Text := <Base.FromDate>` rather
  than `.Farsi_Date := …`, relying on the setter to re-parse.
- Everywhere else (`DKolU`, `DMoein`, `TMoein`, `DaftarT_U`, `RoyatJU`, `Report7U`, `MoeinZipU`,
  `Taraz4Setooni_U`, `Taraz6SetooniU`) uses `.Farsi_Date`.

In the rebuild there is one date type and one serialisation; this distinction disappears.

### 5.6 Fiscal-year scoping

`CO_ID` is a **Jalali fiscal year number** (1397, 1403, …) used as a primary key on `Base` and stamped
onto every transactional row as `M_COID` / `DM_COID` / `AF_COID` / `CM_Coid` / `TM_Coid`. `Base`
carries at least `CO_ID`, `CO_Name`, `CO_Sub`, `FromDate`, `ToDate`, `IsActive`, plus the account-role
pointers `C1081`/`C1081C`/`C1082`/`C1082C` (`Dmu.pas:1068-1136`).

**The global selection** is `DM.CO_ID` (`Dmu.pas:113`, initialised to 0 at `:743`). `DM.From_Date` /
`DM.To_Date` (`Dmu.pas:1138-1150`) resolve the current year's bounds by
`Base.Locate('CO_ID', inttostr(CO_ID), [loCaseInsensitive])` — locating an **integer** field with a
**string** value, relying on Variant coercion. The same pattern appears in `SanDoogh_k`, `Jaryan_K`
and four siblings (`:1073,1086,1095,1108,1121,1130`). `DKolU.pas:138` and `DMoein.pas:227` do it the
other way round, `Locate('CO_ID', Dm.CO_ID, …)` with an integer. Harmless today; unify on a typed key.

**Three different year-scoping behaviours in the reporting surface:**

| Behaviour | Units | Notes |
|---|---|---|
| Per-report picker **with** an "all years" option | `DKolU`, `DMoein`, `TMoein` | see below |
| Per-report picker **without** it | `CardJariU` (`Q2` = `Select * From Base Order By CO_ID`, `.dfm:6540-6541`), `Taraz4Setooni_U` (`Q2`, `.dfm:1330-1331`) | |
| No picker — `DM.CO_ID` only | `DaftarT_U:149`, `KolStateU:103`, `Taraz6SetooniU:73`, `RoyatJU`, `MoeinZipU:257-258`, `MoeinToRU:165` | |

**The "all fiscal periods" escape hatch.** `DKolU.dfm:1193-1203`, `DMoein.dfm:1467-1477` and the
`TMoein` equivalent all use this lookup query:

```sql
Select 0 as CO_ID, Co_name=(select min(co_name) from base), 'همه دوره های مالی' as Co_Sub
     , FromDate=(select min(fromdate) from base) , ToDate=( Select Max(Todate) from base)
Union
Select CO_ID, CO_Name, CO_Sub, FromDate, ToDate
From Base Order By CO_ID
```

Selecting `CO_ID = 0` (`همه دوره های مالی` — "all fiscal periods") makes the ledger emit **no
`M_Coid` predicate at all** (`DKolU.pas:270-273`, `DMoein.pas:641-644`, `TMoein.pas:233-236`) and sets
the date range to `min(FromDate)`..`max(ToDate)` across every year. So the three ledgers can produce a
**cross-year** result. This matters enormously:

- Opening balances then span all history, not one year — which is arguably what a user wants and
  definitely not what the rest of the system assumes.
- It cannot be reconciled against any trial balance, all of which are hard-scoped to one year.
- `Taraz4Setooni_U`'s year picker looks identical but is a **different** query with no synthetic row,
  and (per §2.1(b)) does not even filter data.

The rebuild must model this explicitly: `fiscal_year: Option<FiscalYearId>`, with `None` meaning
all-years, surfaced in the UI as a distinct choice rather than a magic zero.

**`Base.IsActive`** (`Dmu.pas:1008-1014`) archives a year: `Is_New_Sanad_Valid` refuses new vouchers in
an inactive year with `سال مالی مورد نظر بایگانی شده است`. **No report checks `IsActive`** — archived
years remain fully reportable, which is correct behaviour and should be preserved.

**Year-change side effects.** `COID.OnCloseUp` in the ledgers (`DKolU.pas:304-309`, `DMoein.pas:679-685`,
`TMoein.pas:264-269`) resets `D1`/`D2` from the new year's `FromDate`/`ToDate` **and closes the result
set**, discarding any dates the user typed. In `CardJariU` (`.dfm:5931`) it clears the screen and does
*not* reload. Three screens, three behaviours; pick one.

### 5.7 Summary table — date and year filter of every report query verified so far

| Report / unit | Year predicate | Date predicate | `file:line` |
|---|---|---|---|
| 4-column trial balance | `M_COID = DM.CO_ID` (picker ignored) | `M_Date <= @D1` | `Taraz4Setooni_U.pas:103-104` |
| 6-column trial balance | `@Coid = DM.CO_ID` | `@D1`, `@D2` → procedure body | `Taraz6SetooniU.pas:71-73` |
| Daftar Kol opening | `M_Coid = picker` or none | `M_Date < @D1` | `DKolU.pas:270-277` |
| Daftar Kol movement | as above | `M_Date >= @D1 and <= @D2` | `DKolU.pas:284-291` |
| Daftar Moein opening | `M_Coid = picker` or none | `M_Date < @D1` | `DMoein.pas:641-650` |
| Daftar Moein movement | as above | `M_Date >= @D1 and <= @D2` | `DMoein.pas:657-666` |
| Daftar Moein Tajmi'i opening | `M_Coid = picker` or none | `M_Date < @D1` | `TMoein.pas:233-238` |
| Daftar Moein Tajmi'i movement | as above | `M_Date >= @D1 and <= @D2` | `TMoein.pas:245-251` |
| Daftar Tajmi'i | `M_Coid = DM.CO_ID` | `M_date >= @D1 and <= @D2` | `DaftarT_U.pas:149-150` |
| Kol account status | `@CoID = DM.CO_ID` | **none** — whole year | `KolStateU.pas:103` |
| Card Jari account rows | `M_COID = picker` | **none** — whole year | `CardJariU.pas:150` |
| Card Jari final balance | `M_Coid = @Sal` | **none** — whole year | `Dmu.dfm:8677-8683` |
| Journal view (`RoyatJU`) | `Sal` fragment | optional `M_date >= @D1 and <= @D2` | `RoyatJU.pas:301,308` |
| Voucher→journal (`MoeinToRU`) | `M_Coid = DM.CO_ID` | optional `M_Date >= .Text and <= .Text` | `MoeinToRU.pas:164-165` |
| Voucher summary (`MoeinZipU`) | `M_coid = DM.CO_ID` | `M_Date >= @D1 and <= @D2` | `MoeinZipU.pas:258` |

Note the pattern: **the three screens with no date range at all** (Kol account status, both Card Jari
figures) are exactly the three that produce year-to-date numbers users then try to reconcile against
period reports.


---

[← SS4 Card Jari (2/2)](04-04-b-card-jari.md) | [Index](00-index.md) | [SS6 Print pipeline (1/2) →](04-06-a-print-pipeline.md)
