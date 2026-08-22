_Part of [05-inventory](../05-inventory.md) — [index](00-index.md)_

## 11. Stock card and stock balance

Two reports, two entirely different implementations of the same arithmetic. §5.1 already quotes
the balance SQL verbatim and derives the canonical formula; this section covers the screens, the
running-balance algorithm, the ordering question and the gaps.

---

### 11.1 Card Jensi — the item movement card

`AnbarCardJensiU`, reached from `Mainu.pas:549-552` (`AR_Jensi` → `AnbarCardJensi.init`).
It is a **parameter dialog that produces a FastReport**, not a browsable grid.

#### 11.1.1 Parameters and validation

`B_OKClick` (`AnbarCardJensiU.pas:80-124`):

| # | Control | Persian label | Validation | Failure behaviour |
|---|---|---|---|---|
| 1 | `Code` | `کد کالا` item code | `Code.Tag <> 0` — i.e. the code resolved to a real `Anbar_Jens` row in `CodeExit` | **silent `Exit`**, focus moves to `Code` |
| 2 | `D1` | `از تاریخ` from date | `Dm.IsDate(D1.Text)` | **silent `Exit`** |
| 3 | `D2` | `تا تاریخ` to date | `Dm.IsDate(D2.Text)` | **silent `Exit`** |
| 4 | `D1`/`D2` | — | `D1.Text <= D2.Text` — a **string** comparison | **silent `Exit`** |
| 5 | `COID` | `سال مالی` fiscal year | none | — |

> **All four validations fail silently.** No `MessageDlg`, no hint, no colour: the OK button
> appears to do nothing. This is the "handler guarded by a bare `Exit;`" pattern again, four times
> in one procedure. Contrast `Anbar_MandehU.pas:153-193`, which shows a message for every one of
> its six checks. Flag for the rebuild: these need real messages.

`CodeExit` (`:126-147`) resolves the code against `Anbar_Jens` and fills three read-only boxes —
`Name1` (item name), `Vahed` (unit), `Anbar` (the item's home warehouse name, via
`AJ_ID → Anbar_Config.AC_Name`). On failure it blanks them and sets `Code.Tag := 0`.

`sSpeedButton1` opens `AnbarCalaSelect` (the item search, §13) and copies the chosen `AJ_Code`.

**The fiscal year is selectable, and this is the only inventory screen where it is.**
`COID: TDBLookupComboBox` is bound to `Q2` = `Select * From Base Order By CO_ID`
(`AnbarCardJensiU.dfm:880-881`), defaulted to `Dm.CO_ID` (`:65`). So the stock card can be run
for a *previous* fiscal year without switching the application's year — unlike
`Anbar_MandehU`, which is hard-wired to `Dm.CO_ID` (`Anbar_MandehU.pas:196`).

> `Q2`'s design-time connection string points at yet another catalog:
> `Initial Catalog=RPPC; Data Source=PESTEH; User ID=sa` (`AnbarCardJensiU.dfm:872-877`).
> Overwritten at runtime (`:63`). Another credential literal in the binary — §14, and
> `docs/08-platform-and-security.md`.

The default date range is the fiscal year: `D1 := Dm.From_Date`, `D2 := Dm.To_Date` (`:60-61`).

#### 11.1.2 The query

`Dm.SP_AnbarCardJensi` = `Anbar_CardJensi;1` (`Dmu.dfm:453-496`), four parameters:

| Parameter | Type | Bound from |
|---|---|---|
| `@Coid` | int | `COID.KeyValue` — the selected fiscal year |
| `@Code` | int | `Code.Tag` — the resolved item code |
| `@D1` | varchar(10) | `D1.Text` — Jalali `YYYY/MM/DD` |
| `@D2` | varchar(10) | `D2.Text` |

**The procedure body is not in the repository.** Its output columns are recoverable from the
report definition (`AnbarCardJensiU.dfm:489-742, 796, 823, 843`):

| Column | Report use | Inferred meaning |
|---|---|---|
| `AFD_Date` | detail column | movement date (Jalali string) |
| `AFD_Factor` | detail column | invoice number |
| `Sanad` | detail column | voucher number — from `Anbar_Factor.AF_Sanad` |
| `AFD_TypeN` | detail column | document-type label (the `CASE` of §5.1.1) |
| `SunstomerN` | detail column | counterparty name — **spelled `SunstomerN`**, a typo for `CustomerN` baked into the stored procedure's result set. Any rebuild that keeps the SP must keep the typo. |
| `AFD_IN` | detail + `SUM()` footer | inbound quantity (types 1, 4) |
| `AFD_OUT` | detail + `SUM()` footer | outbound quantity (types 2, 3) |
| `AFD_Phi` | detail column | unit price |
| `ssn` | detail column | line id — presumably `AFD_SSN` |
| `Sumr` | detail column | a running or summary figure produced server-side |

So the procedure **pivots one signed movement column into two unsigned columns** (`AFD_IN` /
`AFD_OUT`) using the type mapping of §5.1.1. That is the only place in the codebase where the
direction is materialised as separate columns.

#### 11.1.3 The running-balance algorithm — it lives in the report, not in SQL

`AnbarCardJensiU.dfm:177-195`, the FastReport `PascalScript`:

```pascal
var R:Extended;

procedure Memo9OnBeforePrint(Sender: TfrxComponent);
Var  Bed,Bes : extended;
begin
    Bed := MasterData1.DataSet.Value('AFD_OUT') ;
    Bes := MasterData1.DataSet.Value('AFD_In') ;
    R := R + Bes - Bed ;
    Memo9.Lines.Clear;
    Memo9.Lines.Add( FloatTostr( R )  );
end;

begin
    R := 0;
end.
```

```
running_balance[0] = 0
running_balance[n] = running_balance[n−1] + AFD_IN[n] − AFD_OUT[n]
```

**Five consequences, all of which matter:**

1. **The running balance starts at zero, not at the opening balance.** `R := 0` in the report's
   main block. If `@D1` is anything other than the first day of the year, the "balance" column is
   a running *net movement within the window*, not stock on hand — unless the stored procedure
   emits a synthetic opening row, which the `Sumr` column hints at but does not prove. **Open
   question §14.** This is the first thing to verify against production data.
2. **It is computed at render time, per printed row.** `OnBeforePrint` fires as each detail band
   is laid out. The number therefore depends on the report's traversal, not on the data.
3. **FastReport two-pass rendering would double-count.** `RP_AnbarCardJensi` is shown with
   `ShowReport(True)` (`:119`). If the report is ever switched to two-pass (needed for
   `[TOTALPAGES#]`), every `OnBeforePrint` runs twice and `R` accumulates twice, because `R` is
   only zeroed in the report's `begin…end.` startup block. Latent, not currently triggered.
4. **`R` is `Extended`, printed with `FloatToStr`** — so a balance of `12.75` prints as `12.75`
   but one that arrives at `12.749999999999998` through repeated additions prints in full. There
   is no `DisplayFormat`. Given `AFD_Num` is `Numeric(14,3)` and the additions are binary floating
   point, long cards will show artefacts.
5. **The ordering of same-date movements is whatever the stored procedure returns.** The report
   has no `SortBy`; `MasterData1` walks the dataset in result order. Since the balance is a
   sequential accumulation, **the order is load-bearing** and it is defined in an artefact we
   cannot read.

> **The ordering question, stated precisely.** For two movements on the same `AFD_Date`, the
> plausible tie-breakers are `AFD_Factor` (invoice number) and `AFD_SSN` (line identity). The
> report selects `ssn` as a column, which suggests the SP orders by it. **But `AFD_SSN` is not
> stable across edits** (§5.4): re-saving an invoice deletes and re-inserts all its lines,
> assigning fresh identity values. So if the SP orders by `AFD_SSN`, **editing an old invoice
> moves all of its lines to the end of every stock card for the rest of the year**, silently
> changing the running-balance sequence. If it orders by `AFD_Factor` the sequence is stable but
> wrong whenever invoice numbers are not chronological (which the renumber screen permits, §4.2.6).
> **There is no correct answer available in the current schema.** The rebuild needs an explicit,
> immutable `sequence_no` per movement — see §15.

#### 11.1.4 What the card does not do

- **No warehouse filter.** `AJ_ID` is displayed (`Anbar.Text`, `:140`) but not passed to the SP.
  The card covers the item across all warehouses, consistently with §1.0.
- **No value column beyond `AFD_Phi`.** There is no running value, no moving-average column and no
  cost of issues.
- **No lot/serial column**, because none exists (§1.0).
- **One item at a time.** There is no "all items" mode and no batch print.
- **The report is not closed after printing** (`:121` — `// Close;` is commented out), so the
  dialog stays open for a second run. Deliberate.

---

### 11.2 Mandeh — the stock-balance / activity report

`Anbar_MandehU`, reached from `Mainu.pas:544-547` (`AR_Amalkard` → `Anbar_MandehF.init`).
Form caption `گزارش عملکرد انبار` ("warehouse activity report", `Anbar_MandehU.dfm:5`).

Unlike the stock card this is a **live grid** (`G1: TrDBGrid_MS` over `Q1`) with an optional
FastReport print.

#### 11.2.1 Filters

| Control | Persian label | Default | `.pas` |
|---|---|---|---|
| `C1` | `از کد` from item code | `Min(AJ_Code)` over the whole item master | `:125-127` |
| `C2` | `تا کد` to item code | `Max(AJ_Code)` | `:125-128` |
| `D1` | `از تاریخ` from date | today, clamped into `[Dm.From_Date, Dm.To_Date]`, then **day forced to 1** | `:112-120` |
| `D2` | `تا تاریخ` to date | today, clamped, then **day forced to 31** | `:113-121` |
| fiscal year | — | **not selectable** — hard-wired to `Dm.CO_ID` | `:196` |

> **`D2.Farsi_day := 31` is wrong for eight months of the Jalali year.** Months 1–6 (Farvardin–
> Shahrivar) have 31 days; months 7–11 have 30; Esfand has 29 or 30. Setting day 31 in any month
> from Mehr onward produces an invalid date. The behaviour of `Tools.TFullDate` on an out-of-range
> day is **not verifiable — the control is binary-only and its source is not in the repo**
> (`Tools.pas` is absent; only `Lib.inc` ships). Two possibilities: it clamps to the month length,
> or it leaves `Farsi_Valid = False`, in which case `B_Enteghal1Click`'s check at `:161-166`
> (`تاریخ را به درستی وارد کنید`, "enter the date correctly") fires immediately and the default
> state of the screen is unusable in the second half of the year. **Open question §14** — resolve
> by running the application. Note the same idiom would produce a `'1403/07/31'` string which, in
> the lexicographic comparison of §5.1.2, sorts *after* every real date in Mehr — so even if the
> control accepts it, the report happens to behave as "the whole month".

`D1Change` (`:78-81`) closes `Q1` whenever a date changes, forcing the operator to press
`محاسبه` ("calculate", `B_Calc` → `B_Enteghal1Click`) again. Note it is wired to `D1` only —
changing `D2`, `C1` or `C2` does **not** invalidate the displayed result, so the grid can show
figures that no longer match the visible filters.

#### 11.2.2 Validation — six checks, all with messages

`B_Enteghal1Click` (`Anbar_MandehU.pas:153-204`):

| # | Check | Persian message | English |
|---|---|---|---|
| 1 | `D1.Farsi_Valid` | `تاریخ را  به درستی وارد کنید` | "Enter the date correctly" |
| 2 | `D2.Farsi_Valid` | same | same |
| 3 | `D1 >= DM.From_Date` | `تاریخ باید در رنج سال جاری باشد` | "The date must be within the current year's range" |
| 4 | `D2 <= DM.To_Date` | same | same |
| 5 | `D1 <= D2` | `تاریخ را  به درستی وارد کنید` | "Enter the date correctly" |
| 6 | `C1 > 0` and `C2 > 0` and `C1 <= C2` | `رنج کدها را به درستی وارد کنید` | "Enter the code range correctly" |

Then five parameters are bound and `Q1.Open` runs the SQL of §5.1.2:
`Coid := Dm.CO_ID`, `FCode := C1`, `TCode := C2`, `FDate := D1.Farsi_Date`, `TDate := D2.Farsi_Date`.

#### 11.2.3 Result columns

Eighteen columns (`Anbar_MandehU.dfm:44-186`, field types at `Anbar_MandehU.pas:18-35`):

| Column | Type | Persian title | English | Formula (§5.1.2) |
|---|---|---|---|---|
| `AJ_Code` | int | `کد` | Item code | — |
| `AJ_Name` | string | `نام` | Name | — |
| `AJ_Prop` | string | `مشخصه` | Specification | — |
| `AJ_Vahed` | string | `واحد` | Unit | — |
| `R1` | BCD(14,3) | `اول دوره` | **Opening balance** | `Σ[1,4 before FDate] − Σ[2,3 before FDate]` |
| `Tedin1` | BCD(14,3) | `رسید انبار` | Receipts qty | `Σ AFD_Num[type 1]` in window |
| `Mabin1` | bigint | `مبلغ` | Receipts amount | `Σ AFD_Kol[type 1]` |
| `Phiin1` | bigint | `متوسط` | Receipts average price | `trunc(Mabin1 / Tedin1)` |
| `TedIn2` | BCD(14,3) | `برگشت از فروش` | Sales-return qty | type 4 |
| `Mabin2` / `Phiin2` | | `مبلغ` / `متوسط` | amount / average | type 4 |
| `TedOut1` | BCD(14,3) | `فروش` | Sales qty | type 2 |
| `MabOut1` / `PhiOut1` | | `مبلغ` / `متوسط` | amount / average | type 2 |
| `TedOut2` | BCD(14,3) | `برگشت از خرید` | Purchase-return qty | type 3 |
| `MabOut2` / `PhiOut2` | | `مبلغ` / `متوسط` | amount / average | type 3 |
| `R2` | BCD(14,3) | `مانده نهایی` | **Closing balance** | `R1 + Tedin1 + Tedin2 − TedOut1 − TedOut2` |

**Quantities are `TBCDField` with three decimals here** — the correct precision — in direct
contrast to the `.AsInteger` truncations of §5.2.2 and §6.4. The same figure is read at full
precision in the report and truncated to a whole number in the line editor.

#### 11.2.4 Buttons

| Button | Persian caption | English | Handler | Notes |
|---|---|---|---|---|
| `B_Calc` | `محاسبه` | Calculate | `B_Enteghal1Click` (`:153`) | runs the query |
| `B_Print` | `چاپ لیست` | Print list | `B_PrintClick` (`:138-151`) | FastReport `Rp1`, header text built from the four filters; guarded on `Q1.Active` and `RecordCount > 0` |
| `B_Filter` | `مانده منفی` | Negative balance | `B_FilterClick` (`:213-231`) | toggles a client-side filter `R2<0` — §5.2.3 |
| `B_Exit` | `برگشت` | Back | `:206-211` | resets the filter and closes |

`B_FilterClick` is a real toggle (`B_filter.Tag` flips), and — unlike the "filter controls that are
only ever reset" found elsewhere in this codebase — **it is reachable and it works**. It is the
system's entire negative-stock policy (§5.2.3).

The `B_Print` handler renames two report memos before showing:
`T1` := `<RegName>` + newline + `'گزارش عملکرد انبار'`;
`T2` := `'از تاریخ : …'` / `'تا تاریخ : …'` / `'از کد : …'` / `'تا کد : …'`.

`FormClose` calls `G1.ResetFilter` (`:92`) — the grid's own column filter, distinct from
`Q1.Filter`.

---

### 11.3 The two reports do not agree

| | Card Jensi (`Anbar_CardJensi`) | Mandeh (`Anbar_MandehU.Q1`) |
|---|---|---|
| Scope | one item | a code range |
| Fiscal year | **selectable** | fixed to `Dm.CO_ID` |
| Granularity | one row per movement | one row per item |
| Direction | pivoted into `AFD_IN` / `AFD_OUT` server-side | four separate type buckets |
| Opening balance | **starts at 0** (probably — §11.1.3) | explicit `R1` column |
| Running balance | render-time accumulator in PascalScript | not applicable — closing `R2` computed in SQL |
| Precision | `Extended` float in the report | `Numeric(14,3)` throughout |
| Zero-movement items | shown (they have no rows) | **deleted** — `Delete #R Where R1=0 and R2=0 And Tedin1=0 …` (§5.1.2) |
| Warehouse filter | none | none |
| Implementation | stored procedure, body unavailable | SQL in a `.dfm`, fully readable |

Add the third and fourth implementations from §5.1 (`Anbar_Jens_Phi1`, the dead `Q1` in
`AnbarFactorU.dfm`) and there are **four expressions of "how much stock is there"**, of which one
is unreadable, one is dead, and the two live ones disagree on the date window and on whether the
current invoice counts.

---

### 11.4 Requirements for the rebuild

1. **One balance function**, in the service layer, parameterised by
   `(item, warehouse?, fiscal_year, as_of_date, exclude_document?)`. Everything in §5.1 and §11
   is a call to it.
2. **A stable, immutable movement sequence.** Add `sequence_no` (or use an append-only ledger with
   a monotonic id that survives edits) so the running balance is deterministic and same-date
   ordering is defined. §11.1.3 is unresolvable without it.
3. **The running balance belongs in the query, not in the renderer** — a window function
   `sum(qty_signed) over (partition by item order by movement_date, sequence_no)`.
4. **Exact decimal arithmetic end to end.** `Numeric(14,3)` on the way in, `NUMERIC` in the
   window function, no `Extended`, no `.AsInteger`.
5. **The opening balance must be explicit** on the card, as a first row or a header figure, and it
   must be the opening balance — not zero.
6. **Both reports need a warehouse dimension** once §15's merge of the two subsystems happens.
7. **Replace the four silent `Exit`s** in the Card Jensi dialog with real validation messages.


---

[← 10. Accounting integration (part b)](05-10-b-accounting-integration.md) | [index](00-index.md) | [12. SQL and stored procedures (part a) →](05-12-a-sql-and-stored-procedures.md)
