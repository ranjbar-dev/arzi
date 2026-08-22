_Part of [04-reporting](../04-reporting.md) — [index](00-index.md)_

## 3. General and subsidiary ledgers

Four units render "a ledger". They share a copy-pasted SQL skeleton but differ in what they read
and in one case are **not equivalent**:

| Unit | Persian screen title | Reads | Level | Opening balance? | Running balance? |
|---|---|---|---|---|---|
| `DKolU` | `نمایش دفتر کل` (`DKolU.dfm:5`) | `Moein` where **`M_kind = 2`**, `M_Mo = 0` | one Kol | yes | yes |
| `DMoein` | `نمایش دفتر معین` (`DMoein.dfm:5`) | `Moein` where `M_kind = 1` | one 4-segment account | yes | yes |
| `TMoein` | `مشاهده دفتر معین تجمیعی` (`TMoein.pas:187`) | `Moein`, caller-supplied predicate | arbitrary account set | yes (**buggy**) | yes |
| `DaftarT_U` | `دفتر تجمیعی…` (`DaftarT_U.pas:386-389`) | `Moein` where `M_kind = 1` | multi-select at one level | **no** | **no** |

### 3.0 `M_kind` — the fact that makes Daftar Kol a different animal

`Moein.M_Kind` is not a level marker, it is a **source marker**:

- `M_Kind = 1` — an ordinary subsidiary voucher line (`سند معین`), written by `ArticleMoeinu.pas:157`,
  `SanadEditU.pas:627`, `MakeSanadU.pas:92`, all the treasury units, etc. This is the system of record.
- `M_Kind = 2` — a line of a **manually generated journal-summary voucher** (`سند کل`), written only by
  `ArticleRooznamehU.pas:139` and `MakeRooznamehU.pas:126,154`.

`MakeRooznamehU.B_SaveClick` (`MakeRooznamehU.pas:62-164`) is the generator. Given a voucher-number
range `S1..S2`, a target voucher number `S3` and a date `D1`, it runs

```sql
Select M_Ko , Sum(M_Bed) As Bed, Sum(M_bes) As Bes
From Moein  Where M_Sanad>=<S1> and M_Sanad<=<S2>
 and M_kind=1 and M_COID=<DM.CO_ID>
Group By M_Ko Order By M_ko
```
(`MakeRooznamehU.pas:95-99`) and inserts one `M_Kind = 2` row per non-zero debit total and one per
non-zero credit total, all with `M_Mo = M_Ta1 = M_Ta2 = 0`, `M_Ted = 0`, `M_ID = 0`, `M_Link = 0`,
`M_Code = 0`, **`M_TX = 0`** (`:111-128`, `:139-156`).

Consequences that the rebuild must not inherit silently:

1. **Daftar Kol shows nothing until someone presses "ساخت روزنامه".** It is a report over a
   hand-triggered materialisation, not over the ledger.
2. The dates on Kol-ledger rows are the *operator-chosen* summary date `D1`, not the dates of the
   underlying vouchers. Date filtering in Daftar Kol therefore filters summary dates.
3. The summary lines are always created in state 0 (`M_TX = 0`), so they are drafts until somebody
   confirms voucher `S3` through `SanadMoein.EditSanad(S3, 2)` (`MakeRooznamehU.pas:162`).
4. The completeness check that would have caught a gap in the voucher range is **commented out**
   (`MakeRooznamehU.pas:73-78`): `Q1.RecordCount <> N` is computed and the whole body of the `if` is
   commented, so a range with missing voucher numbers is summarised without warning.
5. Because `M_Kind = 2` rows live in the same table as `M_Kind = 1` rows, **any query that forgets
   `M_kind = 1` double-counts.** One such query exists — see §3.4.

### 3.1 دفتر کل — general ledger (`DKolU`)

**Launched from** `Mainu.pas:667-671` (`TMain.Report9Click` → `DKolF.init`), menu item `Report9`,
caption `دفتر کل` (`Mainu.dfm:10766-10768`). Reachable.

#### Parameter form

| Control | Type | Persian caption | Default | Validation |
|---|---|---|---|---|
| `EKo: TsComboEdit` | int, digits only (`DKolU.pas:203-207`) | `کد حساب کل` (`.dfm:45`) | `''` (`:152`) | **none — see defect below** |
| `SKo: TsEdit` | read-back of `S_Name` | — | filled by `EKoChange` (`:209-219`) | — |
| `BKo: TsSpeedButton` | `?` picker | — | opens `select_Sarfasl.init_Ko` (`:221-227`) | — |
| `COID: TDBLookupComboBox` | fiscal year | `سال مالی` (`.dfm:64`) | `DM.CO_ID` (`:139`) | none |
| `D1: TFullDate` | Jalali from-date | `از تاریخ :` (`.dfm:119`) | `Base.FromDate` of the selected year (`:140`) | `Farsi_Valid` (`:237-241`) |
| `D2: TFullDate` | Jalali to-date | `تا تاریخ :` (`.dfm:130`) | `Base.ToDate` of the selected year (`:141`) | `Farsi_Valid` (`:242-246`), and `D1 > D2` → `Exit` (`:247-251`) |
| `GridFontSize: TsUpDown` | int | — | INI `G1FontSize` | — |

`Q_COID` (`.dfm:1184-1206`) is **not** a plain `Select * From Base`. It prepends a synthetic row:

```sql
Select 0 as CO_ID, Co_name=(select min(co_name) from base), 'همه دوره های مالی' as Co_Sub
     , FromDate=(select min(fromdate) from base) , ToDate=( Select Max(Todate) from base)
Union
Select CO_ID, CO_Name, CO_Sub, FromDate, ToDate
From Base Order By CO_ID
```

`CO_ID = 0` = **`همه دوره های مالی`, "all fiscal periods"**, spanning `min(FromDate)`..`max(ToDate)`.
Selecting it makes `B_OKClick` emit no `M_Coid` predicate at all (`:270-273`), producing a
cross-fiscal-year ledger. This is the only place in the reporting surface where the year scope can be
lifted, and it is the same query in `DMoein.dfm:1467-1477`, `TMoein.dfm` and `CardJariU`. It must be
preserved as an explicit `fiscal_year_id: Option<i32>` in the rebuild.

`COID.OnCloseUp = COIDCloseUp` (`:304-309`) and `DS_BaseDataChange` (`:316-321`) both reset `D1`/`D2`
to the newly selected year's `FromDate`/`ToDate` and close `Qs`, so changing the year discards the
result and the manually typed dates.

#### Permission gate

`B_OKClick:255-260` calls `Dm.Is_Admin_Or_Valid_Daftar(_K, 0, 0, 0)` (`Dmu.pas:921-96x`). That
function returns `Admin` immediately for admins; otherwise it walks Kol → Moein → Tafsil1 → Tafsil2
and returns **false only if some segment's `Sarfasl.S_Lock = 1`** (`Dmu.pas:934,944,954`); a *missing*
`Sarfasl` row returns **true** (`:933,943,953`). The refusal message —
`مشاهده دفتر فقط در اختیار مدیر سیستم است` ("viewing the ledger is reserved for the system
administrator", `DKolU.pas:258`) — is therefore **wrong**: the real rule is a per-account lock flag.
Fix the message, keep the rule.

#### Exact SQL (verbatim, `DKolU.pas:263-299`)

```pascal
Qs.SQL.Add('IF OBJECT_ID(''tempdb..#R'') IS NOT NULL  DROP TABLE #R');

Qs.SQL.Add(' Select 0 as RN, '+ QuotedStr(D1.Farsi_Date)+ ' as M_date, 0 as M_sanad, ''مانده از قبل '' as Article, ' );
Qs.SQL.Add( ' (Sum(M_Bed)) As M_Bed, (Sum(M_Bes)) As M_Bes, 0 as M_Ted, 0 as M_Tx, 0 as M_ID '  );
Qs.SQL.Add('  into #R From moein ');
if Coid.KeyValue>0 then
   Qs.SQL.Add('  Where M_kind=2 and M_Coid='+ inttostr(Coid.KeyValue) )
Else
   Qs.SQL.Add('  Where M_kind=2  ' );

Qs.SQL.Add('  And M_Ko='+ EKo.Text );
Qs.SQL.Add('  And M_Mo=0');
Qs.SQL.Add('  And M_Date <'+ QuotedStr(D1.Farsi_Date) );

Qs.SQL.Add('Union');

QS.SQL.Add('Select ROW_NUMBER() OVER (ORDER BY m_date, M_Sanad) AS RN,  ');
QS.SQL.Add('  M_Date, M_Sanad, Article, M_Bed, M_Bes, M_Ted, M_Tx, M_ID  ');
Qs.SQL.Add('  From moein ');
if Coid.KeyValue>0 then
   Qs.SQL.Add('  Where M_kind=2 and M_Coid='+ inttostr(Coid.KeyValue) )
Else
   Qs.SQL.Add('  Where M_kind=2  ' );
Qs.SQL.Add('  And M_Ko='+ EKo.Text );
Qs.SQL.Add('  And M_Mo=0' );
Qs.SQL.Add('  And M_Date >='+ QuotedStr(D1.Farsi_Date) );
Qs.SQL.Add('  And M_Date <='+ QuotedStr(D2.Farsi_Date) );
Qs.SQL.Add('Order By M_Date, M_Sanad ');

Qs.SQL.Add(' update #R Set M_Bes=0 Where M_Bes is null');
Qs.SQL.Add(' update #R Set M_Bed=0 Where M_Bed is null');
Qs.SQL.Add(' Delete #R Where M_Bed=0 and M_Bes=0 ');

Qs.SQL.Add('  Select (Select Sum(M_Bes-M_Bed) from #R as N Where N.RN<= #R.RN) as Rem, #R.* From #R Order By RN  ');
Qs.Open;
```

#### 3.1.a How the opening balance is computed

The first `SELECT` of the `UNION` is the opening row:

- Predicate: same account, same `M_kind`, same year scope, **`M_Date < D1`** — strictly less than the
  from-date, so an entry dated exactly `D1` belongs to the period, not to the opening. No off-by-one.
- Aggregation: `Sum(M_Bed)` and `Sum(M_Bes)` as **two separate gross totals**. There is **no netting**.
  So the "مانده از قبل" ("balance brought forward") row displays *prior cumulative debit* and *prior
  cumulative credit* side by side; a 1 000 debit and a 1 000 credit before `D1` render as
  `1,000 / 1,000`, not as a zero opening balance.
- Synthetic key columns: `RN = 0`, `M_Date = D1` (the from-date, not the true date of anything),
  `M_Sanad = 0`, `Article = 'مانده از قبل '` (trailing space is in the literal), `M_Ted/M_Tx/M_ID = 0`.
- When the account has no prior activity both sums are `NULL`; `:294-295` coerce them to 0 and
  `:296` deletes the row, so the opening line simply disappears. It also deletes any genuine
  zero-amount movement line.
- `M_Sanad = 0` on the opening row is what disables drill-down for it (`G1DblClick:170-171`).

Netting only happens later, inside the running balance, because `Rem` sums `M_Bes - M_Bed` over all
rows up to and including the current one, opening row included.

#### 3.1.b The running balance

```sql
Select (Select Sum(M_Bes-M_Bed) from #R as N Where N.RN <= #R.RN) as Rem, #R.* From #R Order By RN
```

- `Rem` is **credit-positive**: `Σ(credit − debit)`. A debit-balance account carries a negative `Rem`.
- It is a correlated subquery per row — O(n²) — not a window function, even though the same statement
  already uses `ROW_NUMBER()`.
- The grid (`.dfm:560-566`) shows `Rem` **signed**, `DisplayFormat = '###,###'`.
- The printed report shows `[ ABS(<DB1."Rem">)]` (`.dfm:894`) — **the sign is stripped in print** — and a
  neighbouring memo `D2` is filled by report script `D1OnAfterData` (`.dfm:606-615`):
  ```pascal
  if  <Db1."Rem"> >0 then S:='بس' else S:='بد';
  if  <Db1."Rem"> =0 then S:='';
  D2.memo.text := S;
  ```
  i.e. a two-letter nature indicator under the column header `تش` (`.dfm:777`, short for `تشخیص`):
  `بس` = بستانکار/credit, `بد` = بدهکار/debit, blank at zero. Grid and print therefore express the same
  number two different ways; the rebuild should return `{ amount: i64, side: Debit|Credit }` once.

#### 3.1.c Ordering rule for same-date entries

`ROW_NUMBER() OVER (ORDER BY m_date, M_Sanad)` (`:281`), final `Order By RN` (`:298`).

- Primary key: `M_Date` — a **string** comparison on `'yyyy/mm/dd'` (see §5).
- Secondary key: `M_Sanad`, the voucher number.
- **There is no tie-break below the voucher.** Two lines of the same voucher hitting the same Kol
  account receive `ROW_NUMBER` in an unspecified order, so the intermediate running balances between
  them are not reproducible run to run. `Moein` has an `M_ID`/identity available and it is not used.
  Fix in the rebuild: order by `(date, voucher_no, line_id)`.
- The opening row is forced ahead of everything by `RN = 0`, which works only because `ROW_NUMBER()`
  starts at 1.
- The `UNION` (not `UNION ALL`) at `:279` would collapse duplicate rows, but cannot in practice
  because `RN` is distinct on every movement row.

#### 3.1.d Voucher states included

**All of them.** The `WHERE` clauses at `:270-291` constrain `M_kind`, `M_Coid`, `M_Ko`, `M_Mo` and
`M_Date` and nothing else. `M_Tx` is *selected* (`:282`) and shown in the grid under the caption
`وضعیت` ("status", `.dfm:578-581`) but never filtered. Since `MakeRooznamehU` writes its summary rows
with `M_TX = 0`, **the general ledger is in practice a ledger of draft vouchers.** It reads `Moein`
(truth for its own `M_kind = 2` universe); it never touches the `DMoein` header cache.

#### 3.1.e Output columns

Grid `G1` (`.dfm:208-277`), captions from the persistent fields (`.dfm:541-585`):

| # | Field | Persian caption | English | Format | Align |
|---|---|---|---|---|---|
| 1 | `M_Date` | `تاریخ` | `entry_date` | `varchar(10)` `yyyy/mm/dd` verbatim | centre |
| 2 | `M_Sanad` | `سند` | `voucher_no` | integer | centre |
| 3 | `Article` | `شرح` | `description` | `varchar(250)` | right (RTL) |
| 4 | `M_Bed` | `بدهکار` | `debit` | `'###,###'` — zero renders **empty** | left |
| 5 | `M_Bes` | `بستانکار` | `credit` | `'###,###'` — zero renders empty | left |
| 6 | `Rem` | `مانده` | `running_balance` | `'###,###'`, signed, read-only | left |
| 7 | `M_Ted` | `مقدار` | `quantity` | BCD, precision 18, scale 3 | centre |
| 8 | `M_Tx` | `وضعیت` | `voucher_state` | raw integer 0/1/2 | centre |
| 9 | `M_Id` | `صادر کننده` | `issuer_id` | raw integer | centre |

`'###,###'` has no `0` placeholder, so **zero prints as an empty cell throughout**; there is no
negative-number convention (parentheses or minus) anywhere — negatives only appear in `Rem`, with a
leading `-`.

Report `Rp1` columns (`.dfm:687-853` headers, `.dfm:865-1120` data), right-to-left visual order:
`ردیف` (`[line#]`), `تاریخ سند`, `شماره سند`, `شرح`, `مبلغ بدهکار`, `مبلغ بستانکار`, `مانده`
(`ABS(Rem)`), `تش` (nature letter). Amount memos use `DisplayFormat.FormatStr = '%2.0n'`,
`Kind = fkNumeric`, `HideZeros = True`.

#### 3.1.f Grouping, totals, page breaks

- **No group bands.** Single `MasterData1` (`.dfm:855`) over `DB1` → `QS`. Page A4 portrait
  (`PaperWidth 210`, `PaperHeight 297`, `PaperSize 9`, `.dfm:633-635`). `PageHeader1` repeats the
  column strip on every page; no explicit page break.
- **Footer1** (`.dfm:1121-1175`) carries exactly two grand totals,
  `[SUM(<DB1."M_Bed">,MasterData1)]` and `[SUM(<DB1."M_Bes">,MasterData1)]`. **Both include the
  opening row**, so the printed debit total is *prior cumulative debit + period debit*, i.e. an
  inception-to-`D2` cumulative figure, not a period movement total. There is **no closing-balance
  total** and no debit = credit assertion.
- **Row banding** is `Highlight.Condition = '<line> mod 2 = 1'` with `Fill.BackColor = 15794160`
  applied per memo (`.dfm:889-891` and siblings).

#### 3.1.g Print-time header injection (`DKolU.pas:185-198`)

- `T1` ← `Dm.Base['Co_name'] + CRLF + ' مشاهده دفتر کل ' + CRLF + Trim(COID.Text)`
- `T6` ← `'از تاریخ : ' + D1 + CRLF + 'تا تاریخ : ' + D2 + CRLF + 'صفحه : [Page#]  از [TotalPages#] '`
- `_Name` ← `' حساب کل : ' + EKo.Text + ' ' + SKo.Text`
- The design-time text of `T1` still says `مشاهده دفتر معین` (`.dfm:661-664`) — the layout was
  copy-pasted from `DMoein` — but it is always overwritten at runtime.
- Unlike `DMoein`, `DKolU` does **not** set `_Total`, so the signature block is whatever is baked into
  the layout.

#### 3.1.h Reachability and writes

Reachable (`Mainu.pas:667-671`). **Writes: none** against permanent tables; all DML targets
`tempdb..#R`. `QSBeforeDelete` calls `Abort` (`:160-163`), so the grid cannot delete through the
`ltBatchOptimistic` cursor.

#### 3.1.i Confirmed defects

1. **Empty account code produces a SQL syntax error.** `B_OKClick` never checks `EKo.Text`. With it
   empty, `:275` emits `And M_Ko=` and `Qs.Open` raises. `Is_Admin_Or_Valid_Daftar(0,0,0,0)` returns
   `true` (no `Sarfasl` row with `S_Ko = 0`), so the guard does not stop it. `F_Valid` is declared
   (`:70`), zeroed (`:151`, `:213`) and **never read** — the validity flag that would have caught this
   is dead. Compare `DMoein.B_OKClick:616-620` which does call `Get_Valid`.
2. **The account code is interpolated unescaped** into SQL. `EKoKeyPress` restricts typing to digits
   (`:205-206`), but the picker path `BKoClick` assigns from `Select_Sarfasl._Code` without
   re-validation, and `Q1.Locate` only *looks up* the name, it does not gate the query.
3. Hard-coded design-time connection strings are still in the `.dfm`
   (`DKolU.dfm:517-522` `Initial Catalog=Arzi89;Data Source=MOHSEN-RANJBAR\SQLEXPRESS`,
   `.dfm:1185-1190` `Initial Catalog=RPPC;Data Source=PESTEH`, both `User ID=sa`). They are
   overwritten at runtime from `DM.Ado.ConnectionString` (`:145,149,136`) but are a live credential
   leak in source control — see `08-platform-and-security.md`.
4. `Splitter1`, `sSplitter2`, `Splitter2` and `sGroupBox`-level layout controls are cosmetic;
   `B_Close` and `B_OK` are the only two live buttons besides `S_Print`.

---


---

[← SS2 Trial balances in depth (2/2)](04-02-b-trial-balances-in-depth.md) | [Index](00-index.md) | [SS3 General and subsidiary ledgers (2/3) →](04-03-b-general-and-subsidiary-ledgers.md)
