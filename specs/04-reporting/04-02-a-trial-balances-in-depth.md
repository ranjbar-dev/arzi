_Part of [04-reporting](../04-reporting.md) — [index](00-index.md)_

## 2. Trial balances in depth

Two trial balances exist, and **they are architecturally unrelated**. The 4-column one builds
its numbers in client-generated T-SQL against a `#temp` table; the 6-column one delegates
entirely to a stored procedure whose body is not in this repo. They do not share a single line
of code, a single column name, or the same definition of "turnover". Do not assume that running
both for the same date produces reconcilable output.

There is a third, unused artefact: `DM.SP_Taraz4Setooni` (`Dmu.pas:16`, `Dmu.dfm:34-84`,
`ProcedureName = 'Taraz4Setooni;1'`) is declared on the data module with parameters
`@ToDate varchar(8), @St int, @Ki int, @Sabt int, @Co int` — but **nothing in the repository
ever opens it**. `grep` over all `.pas`/`.dfm` finds only the declaration. The live 4-column
report abandoned the stored procedure for inline SQL; the orphan declaration is dead and must
not be ported.

---

### 2.1 تراز آزمایشی ۴ ستونی — 4-column trial balance (`Taraz4Setooni_U`)

**Launched from** `Mainu.pas:639-643` (`TMain.Report2Click` → `Taraz4Setooni.init`). Reachable.
Form caption `تراز آزمايشي 4 ستوني` (`Taraz4Setooni_U.dfm:5`); on-screen banner
`LTop.Caption` is rebuilt at `Taraz4Setooni_U.pas:215` as `DM.RegName + CRLF + 'تراز آزمایشی 4 ستونی'`.

#### The four columns

| # | Persian caption | Legacy field | English name | Meaning |
|---|---|---|---|---|
| 1 | `گردش بدهکار` | `TBed` | `turnover_debit` | Σ `M_Bed` over all lines with `M_Date <= @D1` |
| 2 | `گردش بستانکار` | `TBes` | `turnover_credit` | Σ `M_Bes` over all lines with `M_Date <= @D1` |
| 3 | `مانده بدهکار` | `RBed` | `balance_debit` | `max(TBed - TBes, 0)` |
| 4 | `مانده بستانکار` | `RBes` | `balance_credit` | `max(TBes - TBed, 0)` |

Grid captions at `Taraz4Setooni_U.dfm:299-323`; print captions at `.dfm:958,977,902,921` under
two group headers `مبلغ گردش به ریال` ("turnover amount in rials", `.dfm:939`) and
`مبلغ مانده به ریال` ("balance amount in rials", `.dfm:883`).

**Critical semantic point:** the caption says *گردش* ("turnover"), but the computation has
**no lower date bound**. `_W` is built once at `Taraz4Setooni_U.pas:103-104` as

```
Where M_Date<='<D1.Farsi_Date>' and M_kind=1 and M_COID=<DM.CO_ID>
```

so column 1/2 are **inception-to-date cumulative debits and credits within the fiscal year**,
not period movement. Because `M_COID` scopes to one fiscal year, "inception" means the start of
that year. The correct English label is `cumulative_debit` / `cumulative_credit`; calling it
turnover is a legacy mislabel that the rebuild should keep in the Persian UI string but not in
the API field name.

#### Exact SQL (verbatim, `Taraz4Setooni_U.pas:95-148`)

```pascal
Q1.SQL.Add(' if Object_ID(''tempdb..#R'' ) is not null Drop Table #R ');
Q1.SQL.Add(' Create Table #R ( St int, CodeStr varchar(30), K int , M int, T1 int, T2 int, Name Varchar(100), TBed bigint, TBes bigint, RBed bigint, RBes Bigint ) ');
Q1.SQL.Add(' Declare @Co int Set @Co='+ inttostr( coid.KeyValue) );
Q1.SQL.Add(' Declare @ML int Set @ML=3 ');

_W := 'Where M_Date<='+QuotedStr(D1.Farsi_Date)+' and M_kind=1 and M_COID='
       + inttostr(DM.CO_ID);
// Kol
Q1.SQL.Add(' insert #R (St,K,M,T1,T2,Name,TBed,TBes,RBed,RBes)');
Q1.SQL.Add(' Select 1, M_Ko, 0,0,0, ''X'' , Sum(M_Bed), Sum(M_Bes), 0 , 0 ');
Q1.SQL.Add(' From Moein '+ _W +' Group By M_Ko ');
// moein
if ST.ItemIndex>0 then
Begin
  Q1.SQL.Add(' insert #R (St,K,M,T1,T2,Name,TBed,TBes,RBed,RBes)');
  Q1.SQL.Add(' Select 2, M_Ko, M_Mo,0,0, ''X'' , Sum(M_Bed), Sum(M_Bes), 0 , 0 ');
  Q1.SQL.Add(' From Moein '+ _W +' Group By M_Ko,M_Mo ');
End;
// Taf1
if ST.ItemIndex>1 then
Begin
  Q1.SQL.Add(' insert #R (St,K,M,T1,T2,Name,TBed,TBes,RBed,RBes)');
  Q1.SQL.Add(' Select 3, M_Ko, M_Mo,M_Ta1,0, ''X'' , Sum(M_Bed), Sum(M_Bes), 0 , 0 ');
  Q1.SQL.Add(' From Moein '+ _W +' and M_ta1>0 Group By M_Ko,M_Mo, M_Ta1 ');
End;
// Taf2
if ST.ItemIndex>2 then
Begin
  Q1.SQL.Add(' insert #R (St,K,M,T1,T2,Name,TBed,TBes,RBed,RBes)');
  Q1.SQL.Add(' Select 4, M_Ko, M_Mo,M_Ta1,M_Ta2, ''X'' , Sum(M_Bed), Sum(M_Bes), 0 , 0 ');
  Q1.SQL.Add(' From Moein '+ _W +' and M_ta2>0 Group By M_Ko,M_Mo, M_Ta1,M_Ta2 ');
End;
// Update name sarfasl
Q1.SQL.Add(' Update #R Set Name=(Select S_name from sarfasl where K=S_ko and M=S_mo and t1=S_ta1 and t2=S_ta2)  ');
Q1.SQL.Add(' Update #R Set Name=''    ''+Name+''    '' Where ST>1 ' );
Q1.SQL.Add(' Update #R Set Name=''    ''+Name+''    '' Where ST>2 ' );
Q1.SQL.Add(' Update #R Set Name=''    ''+Name+''    '' Where ST>3 ' );

Q1.SQL.Add(' Update #R Set RBed=TBed-TBes, RBes=TBes-TBed ');
Q1.SQL.Add(' Update #R Set RBed=0 Where RBed<0 ');
Q1.SQL.Add(' Update #R Set RBes=0 Where RBes<0 ');
/// Plane A
Q1.SQL.Add(' Update #R Set CodeStr = Cast(K as varchar(10) ) ');
Q1.SQL.Add(' Update #R Set CodeStr = Cast(M as varchar(10) )+''   '' where M > 0');
Q1.SQL.Add(' Update #R Set CodeStr = Cast(T1 as varchar(10) )+''      '' where T1 > 0');
Q1.SQL.Add(' Update #R Set CodeStr = Cast(T2 as varchar(10) )+''         '' where T2 > 0');
/// Plane B
Q1.SQL.Add(' Update #R Set CodeStr =  Dbo.Make_R( @Co, K,M,T1,T2) ');

Q1.SQL.Add(' Select * From #R Order By K,M,T1,T2');
Q1.Open;
```

#### How Bed/Bes netting works

`Taraz4Setooni_U.pas:136-138`, three statements, in order:

1. `RBed = TBed - TBes`, `RBes = TBes - TBed` — the two are exact negatives of each other.
2. `RBed = 0 where RBed < 0`
3. `RBes = 0 where RBes < 0`

So the balance pair is a **signed net split into two unsigned columns**: exactly one of
`RBed`/`RBes` is non-zero for any account with a non-zero net; both are zero when the account
nets to zero. There is no debit/credit *nature* check — an expense account with a credit net
simply prints in the credit column. This is the direct consequence of the established fact that
`Sarfasl` carries no account-type column. **Negatives are never displayed**: they are clamped to
zero and moved to the opposite column. There is no parenthesis or minus-sign convention anywhere.

#### Which level it runs at

`ST: TsComboBox` (`Taraz4Setooni_U.dfm:96-111`) — `Style = csDropDownList`, four items:

| `ItemIndex` | Persian | Level produced | `St` rows inserted |
|---|---|---|---|
| 0 (default) | `در سطح کل` | Kol only | 1 |
| 1 | `در سطح معین` | Kol + Moein | 1, 2 |
| 2 | `در سطح تفضیل` | Kol + Moein + Tafsil1 | 1, 2, 3 |
| 3 | `در سطح تفضیل 2` | all four | 1, 2, 3, 4 |

Levels are **cumulative and interleaved in one result set**, not nested bands: each deeper level
is a separate `INSERT` into the same `#R`, and the final `ORDER BY K,M,T1,T2` interleaves them so
a Kol row (`M=0`) sorts immediately before its Moein children. Indentation is faked by prefixing
four spaces to `Name` once per level (`:132-134`), applied cumulatively, so Tafsil2 names carry
12 leading spaces. `ST.ItemIndex > 1` and `> 2` add `and M_ta1>0` / `and M_ta2>0` so accounts
with no analytic segment do not generate degenerate rows.

`OnChange = STChange` (`:265-268`) closes `Q1`, forcing recalculation — the level cannot be
changed without re-pressing محاسبه.

#### Do the totals prove out?

**Report footer** (`Taraz4Setooni_U.dfm:1200-1289`), four totals, each of the form:

```
[SUM( <DB1."RBes"> * IIF(<DB1."St"> >1,  0,1),MasterData1) ]
```

The `IIF(St > 1, 0, 1)` factor **excludes every non-Kol row from the grand total**, which is
exactly right: because the levels are interleaved rather than nested, a naive `SUM` would count
each amount once per level selected. The grand total is therefore always the Kol-level total
regardless of the chosen detail level.

Given that, the proof of balance is:

- Σ`TBed` over Kol rows = Σ`TBes` over Kol rows, **always**, provided every voucher line is
  balanced within itself. This is guaranteed by construction (both are `SUM` over the same row
  set) and does not test anything.
- Σ`RBed` over Kol rows = Σ`RBes` over Kol rows is **not** guaranteed by construction, but is
  implied by the previous identity: since `RBed - RBes = TBed - TBes` per row and the two
  turnover sums are equal, the net sums are equal, and clamping preserves the difference.

So the report is self-proving **only if** `Moein` line-level debits equal credits across the
whole year. It never checks this and never displays a difference. There is **no out-of-balance
warning anywhere in this unit**. That is the single most important defect to fix in the rebuild:
a discrepancy is silently absorbed into the two clamped columns.

There is also an unlabelled second total memo at `Taraz4Setooni_U.dfm:651`,
`[SUM(<DB1."RBes">,MasterData1)]` — the *un-filtered* sum, i.e. counting every level. It lives on
the second, unused report `RP_Kol1`. `RP_Kol1` is declared (`Taraz4Setooni_U.pas:43`) and laid out
(`.dfm:409-735`) but **never printed**; `B_PrintClick` uses `RP_Kol` only (`:167-175`). Dead.

#### Parameter form

| Control | Type | Persian caption | Default | Validation | Effect |
|---|---|---|---|---|---|
| `D1: TEditDate` | Jalali date | `تا تاریخ :` (`.dfm:40`) | `Date()` at `:216`, then overwritten from INI key `Date1` at `:227` | `D1.Farsi_Valid = false` → `MessageDlg('  تاریخ را به درستی وارد کنید  ', mtError)`, focus returns to `D1`, `Exit` (`:82-87`) | `M_Date <= '<value>'` |
| `R0: TsCheckBox` | bool | `اسناد در حال تحریر :` — "vouchers being drafted" (state 0) | INI `TX0`, default 1 (`:228`) | see below | **none** |
| `R1: TsCheckBox` | bool | `اسناد تایید شده :` — "confirmed vouchers" (state 1) | INI `TX1`, default 1 (`:229`) | see below | **none** |
| `R2: TsCheckBox` | bool | `اسناد ثبت دائم شده :` — "permanently posted vouchers" (state 2) | INI `TX2`, default 1 (`:230`) | see below | **none** |
| `ST: TsComboBox` | enum ×4 | (levels, above) | `0`, then INI `state` (`:217,231`) | none | number of `INSERT` blocks |
| `COID: TDBLookupComboBox` | FK to `Base` | (no caption) | `Q2.Last` → **highest** `CO_ID` in `Base` (`:212-213`) | none | see below |
| `F_Size: TComboBox` | enum `سایز 6`..`سایز 13` | | INI `F_size`, default 9 (`:224`) | none | grid + report font size |
| `F_Type: TComboBox` | `اعداد فارسی` / `اعداد انگلیسی` | | INI `F_Type`, default 0 (`:225`) | none | font name only — see §6 |

#### Two defects in the parameter handling — both must be decided before porting

**(a) The three voucher-state checkboxes are dead.** `B_CalcClick:88-93` refuses to run unless at
least one of `R0`/`R1`/`R2` is ticked:

```pascal
if (R0.Checked=false) and (R1.Checked=False) and (R2.Checked=False) Then
Begin
  MessageDlg('  حداقل یکی از سه حالت را انتخاب کنید  ', mterror, [mbok], 0);
  ActiveControl := R2;
  Exit;
End;
```

but `R0`, `R1` and `R2` appear **nowhere else in the unit** except this guard and the INI
persistence at `:190-192`. The `WHERE` clause at `:103-104` filters on `M_Date`, `M_kind` and
`M_COID` and nothing else. **The 4-column trial balance therefore always includes vouchers in
every state, including unposted drafts (state 0).** The checkboxes are decoration that only
serve to block the button when all three are cleared. The rebuild must choose: implement the
filter the UI promises, or delete the controls. Do not silently keep both.

The INI persistence is itself buggy — `:191` and `:192` write to key `'TX0'` on the false branch
instead of `'TX1'`/`'TX2'`:

```pascal
if R1.Checked then MyINI.WriteInteger(Name,'TX1',1) Else MyINI.WriteInteger(Name,'TX0',0);
if R2.Checked then MyINI.WriteInteger(Name,'TX2',1) Else MyINI.WriteInteger(Name,'TX0',0);
```

so unticking R1 or R2 clears R0's saved state instead of their own.

**(b) The fiscal-year picker does not pick the fiscal year.** `@Co` is set from
`coid.KeyValue` (`:100`) but the data filter uses `DM.CO_ID` (`:104`) — the globally selected
fiscal year from the data module. `@Co` is consumed only by the last `CodeStr` update
(`:145`, `Dbo.Make_R(@Co, K,M,T1,T2)`), i.e. it affects **only how the account code string is
formatted**, never which rows are read. Selecting fiscal year 1397 in the dropdown while the
application is on 1399 produces 1399 numbers with 1397-style code strings. `COID` is
additionally initialised to `Q2.Last` (`:212-213`, `Q2.SQL = 'Select * From Base Order By CO_ID'`,
`.dfm:1330-1331`), i.e. the newest year — which is *not* necessarily `DM.CO_ID`. So out of the
box the two disagree.

**(c) `B_Save` (`ذخیره`, "save") has no handler.** Declared at `Taraz4Setooni_U.pas:27`, laid out
at `.dfm:216-230` with `Enabled = False` and no `OnClick`. Dead; drop it.

#### Writes performed

None. The only DML is against `tempdb..#R`. `SQ: TADOQuery` (`:29`, `.dfm:326-330`) has no SQL
and is never used — dead.

#### `Dbo.Make_R` — an undocumented scalar function

`Dbo.Make_R(@Co, K, M, T1, T2)` returns the display account code (`Taraz4Setooni_U.pas:145`;
also used in `RoyatJU.pas:367` with a hard-coded `1` for `@Co`, and in a commented-out line at
`Dmu.pas:278`). **Its body is not in this repo.** It is the authority on how a 4-segment code is
rendered as a string — `CodeStr varchar(30)`, and `RoyatJU` writes its output into `M_L`. The
"Plane A" block at `:140-143` is a hand-rolled fallback that the very next line overwrites
unconditionally; it is effectively dead code but documents the intended shape: the code string is
the *deepest non-zero segment* padded with trailing spaces proportional to depth, not a
dotted concatenation.

#### Grouping, sorting, page breaks, formatting

- **Sort:** `Order By K,M,T1,T2` (`:147`). Numeric, not string — so account `10` sorts before
  `9`… no: `K` is `int`, so `9` before `10`. Correct numeric ordering.
- **Grouping:** none in FastReport terms. A single `MasterData1` band (`.dfm:1021`) over a flat
  interleaved result set. Level is conveyed by the `St` column, the name indentation, and row
  colour.
- **Row colour** is computed in the report's PascalScript `D1OnAfterData`
  (`Taraz4Setooni_U.dfm:748-797`): alternating banding *per level*, `St=1` → `$DDDDC4`/`$EEEED4`,
  `St=2` → `$E0FFFF`/`$CDFAFF`, `St=3` → `$FFFFFF`/`$F0F0F0`. `St=4` gets no case and falls
  through to plain white. Counters `LK/LM/LT1/LT2` reset each other so banding restarts inside
  each parent.
- **Number format:** grid uses `DisplayFormat = '#,###'` on all four amount fields
  (`.dfm:383,389,395,401`) — thousands separated, **no decimals, and zero renders as empty
  string** (`#,###` with no `0` placeholder). Report memos use
  `DisplayFormat.FormatStr = '%2.0n'`, `Kind = fkNumeric`, `DecimalSeparator = '/'`
  (`.dfm:1159-1161` and siblings) plus `HideZeros = True`. Amounts are `bigint` rials, integral.
- **Page:** A4 portrait, `PaperWidth = 210`, `PaperHeight = 297`, `PaperSize = 9`
  (`.dfm:812-814`). No explicit page-break rule; overflow paginates naturally on the
  `MasterData1` band. `PageHeader1` (`.dfm:821`) repeats the column headings on every page.
- **Print-time header injection** (`Taraz4Setooni_U.pas:171-174`):
  - `T1` ← `DM.RegName + CRLF + COID.Text + CRLF + '  تراز آزمایشی 4 ستونی  ' + ST.Text`
  - `T2` ← `' تا تاريخ : ' + D1.Farsi_Date + CRLF + '  صـفـحـه : [Page#] از [totalPages#]  '`
  - `_Total` ← `DM.Get_paramstr(1014)` (`Dmu.pas:468`) — the signature block; laid out as
    `تنظیم کننده: … مدیرمالی: … مدیرعامل:` in the static fallback at `.dfm:1310-1313`.
  - `D1`..`D11` get `Font.Size := F_Size.ItemIndex+6` and
    `Font.Name := 'B Yekan' | 'WeblogmaYekan'` in a loop (`:166-169`).

---


---

[← SS1 Report catalogue (3/3)](04-01-c-report-catalogue.md) | [Index](00-index.md) | [SS2 Trial balances in depth (2/2) →](04-02-b-trial-balances-in-depth.md)
