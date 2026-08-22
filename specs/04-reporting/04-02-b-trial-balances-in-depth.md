_Part of [04-reporting](../04-reporting.md) — [index](00-index.md)_

### 2.2 تراز آزمایشی ۶ ستونی — 6-column trial balance (`Taraz6SetooniU`)

**Launched from** `Mainu.pas:650-653` (`TMain.Report5Click` → `Taraz6Setooni.init`). Reachable.
Form caption `تراز آزمایشی 6 ستونی` (`Taraz6SetooniU.dfm:5`), `BiDiMode = bdRightToLeft`.

This form has **no grid**. Pressing `مشاهده تراز` opens the stored proc and goes straight to the
FastReport preview (`Taraz6SetooniU.pas:96-103`). There is no on-screen tabular view at all.

#### The six columns

Confirmed from the two-tier page header and the `Left` coordinates of each memo. FastReport
`Left` is physical-left; the page is laid out right-to-left, so the rightmost (first-read) column
has the largest `Left`.

| Visual order (RTL) | `Left` | Group header | Column header | Data field | English name |
|---|---|---|---|---|---|
| 1 | 948.66 | — | `کد` (`.dfm:438`) | composed expression, see below | `account_code` |
| 2 | 680.31 | `مشخصات` (`.dfm:807`) | `نام کد` (`.dfm:458`) | `S_name` | `account_name` |
| 3 | 566.93 | `گردش قبل از دوره` (`.dfm:768`) | `گردش بدهکار` (`.dfm:748`) | **`Bed1`** | `opening_turnover_debit` |
| 4 | 453.54 | `گردش قبل از دوره` | `گردش بستانکار` (`.dfm:728`) | **`Bes1`** | `opening_turnover_credit` |
| 5 | 340.16 | `گردش طی دوره` (`.dfm:708`) | `گردش بدهکار` (`.dfm:534`) | **`Bed2`** | `period_turnover_debit` |
| 6 | 226.77 | `گردش طی دوره` | `گردش بستانکار` (`.dfm:514`) | **`Bes2`** | `period_turnover_credit` |
| 7 | 113.39 | `مانده پایان دوره` (`.dfm:787`) | `مانده بدهکار` (`.dfm:418`) | `RBed` | `closing_balance_debit` |
| 8 | 0.00 | `مانده پایان دوره` | `مانده بستانکار` (`.dfm:398`) | `RBes` | `closing_balance_credit` |

**Naming trap — read carefully.** The `TfrxMemoView` *component names* are shifted by one
relative to the *fields they display*:

| Component | Displays | Actually is |
|---|---|---|
| `Bed` (`.dfm:953`) | `[DB."Bed1"]` | prior-period debit |
| `Bes` (`.dfm:927`) | `[DB."Bes1"]` | prior-period credit |
| `Bed1` (`.dfm:871`) | `[DB."Bed2"]` | in-period debit |
| `Bes1` (`.dfm:844`) | `[DB."Bes2"]` | in-period credit |
| `RBed` (`.dfm:1005`) | `[DB."RBed"]` | closing debit |
| `RBes` (`.dfm:979`) | `[DB."RBes"]` | closing credit |

and the footer totals are named `BedT`/`BesT` for the **in-period** pair and `TBed`/`TBes` for
the **prior** pair (`.dfm:1038-1132`), i.e. reversed relative to the intuitive reading. Do not
port these names.

So the 6 columns are: **opening turnover (Bed/Bes) + period turnover (Bed/Bes) + closing balance
(Bed/Bes)**. Note this is *turnover* opening, not an opening *balance* pair — the first two
columns are gross accumulations before `@D1`, unnetted. That differs from the common
Iranian "تراز ۶ ستونی" convention where columns 1-2 are the opening *balance*; whether the
stored procedure nets them is **unverifiable from this repository** (see §9).

The account-code cell is composed in the report, not the query (`Taraz6SetooniU.dfm:915-918`):

```
[DB."S_ko"][iif( <DB."S_Mo"> >0 , '-' +inttostr(<DB."S_Mo">), '' )][iif( <DB."S_Ta1"> >0 , '-' +inttostr(<DB."S_Ta1">), '' )][iif( <DB."S_Ta2"> >0 , '-' +inttostr(<DB."S_Ta2">), '' )]
```

i.e. hyphen-joined segments, omitting zero segments. **This is a different code format from the
4-column report's `Dbo.Make_R`.** Two trial balances, two incompatible account-code renderings.

#### Stored-procedure call signature

`DM.SP_Taraz6Setooni` (`Dmu.pas:45`, `Dmu.dfm:751-808`), `ProcedureName = 'Taraz_6Sotooni;1'`.
Note the procedure is named **`Taraz_6Sotooni`** — underscore, different transliteration — while
the Delphi component is `SP_Taraz6Setooni`. Grep the live database for the former.

| Parameter | Type | Design-time default | Set at |
|---|---|---|---|
| `@RETURN_VALUE` | int, return | 0 | — |
| `@D1` | `varchar(10)` | `'1397/09/01'` | `Taraz6SetooniU.pas:71` ← `D1.Farsi_Date` |
| `@D2` | `varchar(10)` | `'1397/09/31'` | `:72` ← `D2.Farsi_Date` |
| `@kind` | int | 1 | `:76-78` |
| `@Coid` | int | `1397` | `:73` ← `DM.CO_ID` |
| `@Level` | int | 3 | `:80-86` |
| `@Sabt` | int | 3 | `:89-94` |

The design-time defaults are themselves evidence: `@Coid = 1397` and `@D1 = '1397/09/01'`
independently confirm that `CO_ID` is a Jalali fiscal year and that dates are stored as
zero-padded `yyyy/mm/dd` strings, 10 characters.

**`@kind` — نوع اسناد ("voucher kind"), `Taraz6SetooniU.pas:75-78`:**

```pascal
if Rx0.Checked
   Then Dm.SP_Taraz6Setooni.Parameters.ParamByName('@Kind').Value := 2
   Else Dm.SP_Taraz6Setooni.Parameters.ParamByName('@Kind').Value := 1 ;
```

`Rx0` is captioned `تراز آزمايشي 6 ستوني از روي اسناد روزنامه` — "6-column trial balance **from
the journal vouchers**" (`.dfm:159`). So `@kind = 2` means "read the journal/`Rooznameh` source"
and `@kind = 1` means the default source. Which physical tables those select is inside the
procedure body and cannot be determined here.

**`@Level` — نوع تراز ("trial-balance type"), `:79-86`:** initialised to `1` unconditionally,
then overwritten:

| Radio | Persian caption (`.dfm`) | `@Level` |
|---|---|---|
| `RX1` | `تراز آزمايشي 6 ستونی در سطح کل` (`:151`) | 1 |
| `RX2` | `تراز آزمايشي 6 ستونی در سطح معین` (`:143`) | 2 |
| `RX3` | `تراز آزمايشي 6 ستونی در سطح 1 تفضیل` (`:135`) | 3 |
| `RX4` | `تراز آزمايشي 6 ستونی در سطح 2 تفضیل` (`:167`) | 4 |
| `RX0` | `تراز آزمايشي 6 ستوني از روي اسناد روزنامه` (`:159`) | **1 (falls through)** |

`RX0` is in the same radio group but sets no level, so selecting it yields `@Level = 1` *and*
`@kind = 2`. That is almost certainly intentional (journal-sourced balance at Kol level), but it
is implicit and undocumented.

**`@Sabt` — ثبت و تاييد شده ("posted and confirmed"), `:88-94`:**

```pascal
if ch1.Checked then ...'@Sabt').Value := 1;
if ch2.Checked then ...'@Sabt').Value := 2;
if ch1.Checked and ch2.Checked then ...'@Sabt').Value := 3;
```

with `CH1` = `اسناد تاييد شده` ("confirmed vouchers", `.dfm:38`) and `Ch2` = `اسناد ثبت شده`
("posted/registered vouchers", `.dfm:47`). So `@Sabt` is a bitmask: 1 = confirmed only,
2 = posted only, 3 = both. **Unlike the 4-column report, this filter is real** — it is passed to
the procedure. What the procedure does with it is unverifiable here.

Mapping to the established state machine `0 → 1 → 2`: `CH1`/`@Sabt=1` ≈ state 1 (تایید /
confirmed), `Ch2`/`@Sabt=2` ≈ state 2 (ثبت / permanently posted). **There is no way to include
state 0 drafts**, and no checkbox for them. So the 6-column trial balance excludes drafts and the
4-column one includes them — the two reports disagree on scope by design.

Because `init` sets both boxes (`:136-137`), the default is `@Sabt = 3`.

#### Parameter form

| Control | Type | Persian caption | Default | Validation |
|---|---|---|---|---|
| `D1: TFullDate` | Jalali date | `از تاریخ` (`.dfm:86`) | `Date()` then `Farsi_day := 1` (`:131-132`) → 1st of current Jalali month | `not D1.Farsi_Valid` → focus `D1`, `Exit` (`:56-60`) |
| `D2: TFullDate` | Jalali date | `تا تاریخ` (`.dfm:99`) | `Date()` then `Farsi_day := 31` (`:133-134`) | **see bug below** |
| `RX0..RX4` | radio group | (above) | `RX2` checked (`:135`) | none |
| `CH1` | bool | `اسناد تاييد شده` | checked (`:136`) | at least one of CH1/CH2 (`:51-55`) |
| `Ch2` | bool | `اسناد ثبت شده` | checked (`:137`) | as above |

**Bugs in the validation block (`Taraz6SetooniU.pas:56-69`):**

1. `D2` is never validated. Line 61 repeats `if not d1.Farsi_Valid` and only the
   `ActiveControl` assignment mentions `d2`:
   ```pascal
   if not d1.Farsi_Valid then
   Begin
      ActiveControl := d2;
      Exit;
   End;
   ```
   A malformed `D2` proceeds straight into the query.
2. The `D1 > D2` check at `:66-69` sets `ActiveControl := D1` but **does not `Exit`**. An inverted
   range runs anyway and returns an empty period.
3. `D2.Farsi_day := 31` produces `'…/…/31'` for months that have 30 days (Mehr–Esfand) and for
   Esfand (29 or 30). As an *upper* string bound this is harmless — `'1399/08/31' > '1399/08/30'`
   — but it means the default `D2` is frequently a date that does not exist, and any code that
   round-trips it through a real calendar will reject it.

#### Grouping, totals, formatting

- **Bands:** one `MasterData1` over `DB` → `DM.SP_Taraz6Setooni` (`.dfm:812-819`,
  `.dfm:1212-1215`). No group bands, no page-break rule. `PageHeader1` (`.dfm:303`) repeats the
  two-tier heading.
- **Grand totals** in `Footer1` (`.dfm:1032-1183`), all six columns, each of the form
  ```
  [SUM( IIF(<DB."S_mo"> =0, <DB."Bes2">,0),MasterData1)]
  ```
  i.e. **only rows with `S_Mo = 0` (Kol-level rows) contribute**, the same anti-double-count
  device as the 4-column report but keyed on `S_Mo` rather than a level column. This implies the
  procedure returns a mixed-level result set with Kol rows carrying `S_Mo = 0`.
- **Row shading** in PascalScript `Bes1OnAfterData` (`.dfm:185-237`): every cell of a row with
  `S_Mo = 0` is painted `cl3DLight`, all others `clWhite`. So Kol summary rows are visually
  greyed. A large commented-out block (`.dfm:238-272`) shows an abandoned attempt to switch
  `DisplayFormat` per row based on a `Real_Len` field — evidence that a decimal/integer toggle was
  once planned and dropped.
- **Number format:** `DisplayFormat.FormatStr = '%2.0n'`, `Kind = fkNumeric`,
  `DecimalSeparator = '/'`, `HideZeros = True` on all six amount memos and all six totals. Zeros
  print blank. No negatives are expected (same clamped-pair model).
- **Print-time injection** (`Taraz6SetooniU.pas:97-102`): `_D1` ← `D1.Farsi_Date`,
  `_D2` ← `D2.Farsi_Date`, `_T1` ← `DM.RegName`, `_Total` ← `DM.Get_paramstr(1014)`. `_T2` and
  `_T3` exist in the layout but their assignments are commented out (`:100-101`) — `_T2` carries
  the static text `تراز آزمایشی 6 ستونی` (`.dfm:672`), `_T3` and `_D3` render blank at runtime.
  Static labels `از سند :` / `تا سند :` (`.dfm:361,379`) sit in the header with **no
  corresponding voucher-number filter on the form** — a leftover from a from-voucher/to-voucher
  variant that no longer exists.
- `Rp_TarazMoein.ShowReport(true)` (`:103`) — modal preview, see §6.

#### Writes performed

None from Delphi. Whether `Taraz_6Sotooni` writes is unknown — the body is not in this repo. It
must be inspected before the rebuild trusts it as read-only (see §9).

---

### 2.3 Reconciliation risk summary

| Aspect | 4-column | 6-column |
|---|---|---|
| Data source | inline SQL on `Moein` | `Taraz_6Sotooni` (body unknown) |
| Filter | `M_kind = 1`, `M_COID`, `M_Date <=` | `@kind`, `@Coid`, `@D1`/`@D2`, `@Sabt` |
| Voucher states included | **all, incl. drafts** | confirmed and/or posted only |
| Reads truth or cache | `Moein` (truth) | unknown |
| Account-code rendering | `Dbo.Make_R` | hyphen-joined in the report |
| Level selection | cumulative (Kol..selected) | single `@Level` value |
| Opening columns | none | gross prior turnover |
| Grand total device | `IIF(St > 1, 0, 1)` | `IIF(S_mo = 0, …, 0)` |
| Out-of-balance detection | none | none |

The rebuild should produce **one** trial-balance engine with an explicit `as_of` / `from`..`to`
mode switch, an explicit voucher-state set, and a mandatory balance assertion, and render both
legacy layouts from it.


---

[← SS2 Trial balances in depth (1/2)](04-02-a-trial-balances-in-depth.md) | [Index](00-index.md) | [SS3 General and subsidiary ledgers (1/3) →](04-03-a-general-and-subsidiary-ledgers.md)
