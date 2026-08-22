_Part of [04-reporting](../04-reporting.md) — [index](00-index.md)_

### 3.2 دفتر معین — subsidiary ledger (`DMoein`)

**Launched from** `Mainu.pas:636-639` (`TMain.Report1Click` → `DMoeinF.init(0,0,0,0)`), menu item
`Report1`, caption `مشاهده دفتر معين` (`Mainu.dfm:10712-10716`). Also opened as a **drill-down** from
`CardJariU.pas:390` and from the journal viewer `RoyatJU.pas:142,144`. Reachable through all three.

Signature: `init(C_Ko, C_Mo, C_Ta1, C_Ta2: integer; _COID: integer = 0; _D1: String = ''; _D2: String = '')`
(`DMoein.pas:220`). When called with a non-zero `C_Ko` it pre-fills the code boxes, runs
`B_OKClick` itself (`:279`) and **returns without showing the form** if the result set is empty
(`:281-284`) — so a drill-down onto an account with no movement in range silently does nothing.

#### Parameter form

Four cascading code boxes, each with a `?` picker and a read-only name box:

| Control | Persian caption | Lookup query | Enabled when |
|---|---|---|---|
| `EKo` / `SKo` / `BKo` | `کل:` (`.dfm:44`) | `Q1`: `Select * From Sarfasl Where S_Ko>0 And S_Mo=0` (`.dfm:643-644`) | always (`:258`) |
| `EMo` / `SMo` / `BMo` | `معین:` (`.dfm:72`) | `Q2`: `… Where S_Ko=:Ko And S_Mo>0 and S_Ta1=0` (`.dfm:660-661`) | `F_Valid=0 and F_Ko>0` (`:439-440`) |
| `ETa1` / `STa1` / `BTa1` | `تفضیل 1:` (`.dfm:91`) | `Q3`: `… Where S_Ko=:Ko And S_Mo=:Mo and S_Ta1>0 and S_Ta2=0` (`.dfm:686-688`) | `F_Valid=0 and F_Mo>0` (`:494-495`) |
| `ETa2` / `STa2` / `BTa2` | `تفضیل 2:` (`.dfm:110`) | `Q4`: `… and S_Ta2>0` | `F_Valid=0 and F_Ta1>0` (`:523-524`) |
| `COID` | `سال مالی` (`.dfm:253`) | same `Base` + synthetic `CO_ID = 0` union as §3.1 (`.dfm:1467-1477`) | — |
| `D1` / `D2` | `از تاریخ :` / `تا تاریخ :` (`.dfm:231,242`) | `Base.FromDate`/`ToDate`, overridable by the `init` arguments (`:229-232`) | — |

`F_Valid` here **is** used: it is set to 1 as soon as the next level down has no children
(`:492`, `:521`, `:543`), which is this unit's stand-in for the missing `S_Child = 0` leaf test, and
`B_OKClick:616-620` refuses to run unless `Get_Valid` is true. So `DMoein` will only run on a **leaf**
account, whereas `DKolU` will run on any Kol number.

`Set_FullCode` (`:305-348`) parses a hyphen-joined `k-m-t1-t2` string into the four boxes;
`Get_FullCode` (`:164-173`) rebuilds it; `Get_FullName` (`:175-184`) joins the four names with `/`.
Note the code format here is **hyphen-joined**, matching the 6-column trial balance and *not*
`Dbo.Make_R`.

`BKo/BMo/BTa1/BTa2` are `Visible = False` until the matching edit gains focus (`:443-466`,
`:547-555`) — a hover-reveal, not dead UI.

#### Exact SQL (verbatim, `DMoein.pas:634-674`)

```pascal
Qs.SQL.Add('IF OBJECT_ID(''tempdb..#R'') IS NOT NULL  DROP TABLE #R');

Qs.SQL.Add(' Select 0 as RN, '+ QuotedStr(D1.Farsi_Date)+ ' as M_date, 0 as M_sanad, ''مانده از قبل '' as Article, ' );
Qs.SQL.Add( ' (Sum(M_Bed)) As M_Bed, (Sum(M_Bes)) As M_Bes, 0 as M_Ted, 0 as M_Tx, 0 as M_ID '  );
Qs.SQL.Add('  into #R From moein ');
if Coid.KeyValue>0 then
   Qs.SQL.Add('  Where M_kind=1 and M_Coid='+ inttostr(Coid.KeyValue) )
Else
   Qs.SQL.Add('  Where M_kind=1  ' );

Qs.SQL.Add('  And M_Ko=0'+ EKo.Text );
Qs.SQL.Add('  And M_Mo=0'+ EMo.Text );
Qs.SQL.Add('  And M_Ta1=0'+ ETa1.Text );
Qs.SQL.Add('  And M_Ta2=0'+ ETa2.Text );
Qs.SQL.Add('  And M_Date <'+ QuotedStr(D1.Farsi_Date) );

Qs.SQL.Add('Union');

QS.SQL.Add('Select ROW_NUMBER() OVER (ORDER BY m_date, M_Sanad) AS RN,  ');
QS.SQL.Add('  M_Date, M_Sanad, Article, M_Bed, M_Bes, M_Ted, M_Tx, M_ID  ');
Qs.SQL.Add('  From moein ');
if Coid.KeyValue>0 then
   Qs.SQL.Add('  Where M_kind=1 and M_Coid='+ inttostr(Coid.KeyValue) )
Else
   Qs.SQL.Add('  Where M_kind=1  ' );
Qs.SQL.Add('  And M_Ko=0'+ EKo.Text );
Qs.SQL.Add('  And M_Mo=0'+ EMo.Text );
Qs.SQL.Add('  And M_Ta1=0'+ ETa1.Text );
Qs.SQL.Add('  And M_Ta2=0'+ ETa2.Text );
Qs.SQL.Add('  And M_Date >='+ QuotedStr(D1.Farsi_Date) );
Qs.SQL.Add('  And M_Date <='+ QuotedStr(D2.Farsi_Date) );
Qs.SQL.Add('Order By M_Date, M_Sanad ');

Qs.SQL.Add(' update #R Set M_Bes=0 Where M_Bes is null');
Qs.SQL.Add(' update #R Set M_Bed=0 Where M_Bed is null');
Qs.SQL.Add(' Delete #R Where M_Bed+M_Bes=0 ');

Qs.SQL.Add('  Select (Select Sum(M_Bes-M_Bed) from #R as N Where N.RN<= #R.RN) as Rem, #R.* From #R Order By RN  ');
Qs.Open;
```

**`And M_Ko=0' + EKo.Text` is deliberate, not a typo.** The literal `0` is a prefix so that an empty
edit box yields `M_Ko=0` (matches nothing) and a filled one yields `M_Ko=011` — which SQL Server
parses as the integer `11`. It works only because `EKoKeyPress` (`:408-412`) restricts input to
digits. It is unreadable and brittle; in the rebuild use a nullable parameter.

Everything else — opening-balance rule (`M_Date < D1`, gross unnetted sums, `RN = 0`), running-balance
formula (`Σ(M_Bes − M_Bed)` over `RN <= current`, credit-positive), ordering rule
(`m_date, M_Sanad`, no third tie-break), and the absence of any `M_Tx` filter — is **identical to
§3.1.a–d**. The only differences are `M_kind = 1` instead of `2`, the four-segment account predicate,
and `Delete #R Where M_Bed+M_Bes=0` instead of `M_Bed=0 and M_Bes=0` (equivalent for non-negative
amounts).

**Voucher states included: all, including state-0 drafts.** This is a real reconciliation hazard —
the subsidiary ledger and the 6-column trial balance (which excludes drafts, §2.2) will not agree.

#### Output columns

Same nine columns, same captions, same `'###,###'` formats as §3.1.e (`.dfm:346-407` grid,
`.dfm:753-796` fields). Same report script `D1OnAfterData` producing `بس`/`بد` (`.dfm:817-829`).
Same A4 portrait page (`.dfm:846`).

#### Print-time header injection (`DMoein.pas:350-406`)

Builds three parallel strings and pushes them into three stacked memos, so the header renders as a
little table:

- `T3` ← `': کل' + CRLF + ': معین'` (+ `': تفضیل1'`, `': تفضیل2'` when those levels are filled)
- `T4` ← the code values, one per line
- `T5` ← the account names, one per line
- `T1` ← `Base['Co_name'] + CRLF + ' مشاهده دفتر معین ' + CRLF + Trim(COID.Text)`
- `T6` ← from/to dates + `صفحه : [Page#]  از [TotalPages#]`
- `_Total` ← **`Dm.Get_paramstr(1013)`** (`:401-402`) — the signature block. Note the trial balances
  use parameter **1014** (`Taraz4Setooni_U.pas:174`); the ledgers use **1013**. Two separate
  configurable footers, see §6.

#### Reachability and writes

Reachable. **Writes: none**; `QSBeforeDelete` aborts (`:289-292`); all DML is on `tempdb..#R`.

#### Drill-down

`G1DblClick` (`:294-303`) → `SanadEditF.View(M_Sanad, COID.KeyValue)` — opens the voucher in the
voucher editor in view mode. Guarded against `M_Sanad = 0`, which is exactly the opening row.

---

### 3.3 دفتر معین تجمیعی — consolidated subsidiary ledger (`TMoein`)

**Launched only from `CardJariU.pas:426`** (`TMoeinF.init(_SW, _SC, _SN, COID.KeyValue)`). It has no
menu entry of its own. Signature `init(DW, DC, DN: String; _COID: integer = 0)` (`TMoein.pas:118`):

- `DW` → private field `_Where`, a **raw SQL predicate fragment** built by the caller;
- `DC` → memo `M1`, the code caption to print;
- `DN` → memo `M2`, the name caption to print.

Layout, grid columns, formats, running balance, drill-down (`:153-159`) and report are the same as
`DMoein`. Two things differ.

**(a) It is a SQL-fragment injection point.** `_Where` is spliced verbatim into both legs
(`:237`, `:249`). Whatever `CardJariU` composes becomes part of the query. See §4 for what it composes.

**(b) The opening-balance leg omits `M_kind = 1` — a real double-count bug.**

```pascal
// opening leg, TMoein.pas:233-238
if Coid1.KeyValue>0 then
   Qs.SQL.Add('  Where M_Coid='+ IntToStr(COID1.KeyValue)+ ' and ' )
Else
   Qs.SQL.Add('  Where  ' );
QS.SQL.Add('  ( ' + _Where + ' )' );
Qs.SQL.Add('  And M_Date <'+ QuotedStr(D1.Farsi_Date) );

// movement leg, TMoein.pas:245-249  — this one DOES filter
if Coid1.KeyValue>0 then
   Qs.SQL.Add('  Where M_kind=1 and M_Coid='+ IntToStr(COID1.KeyValue) )
Else
   Qs.SQL.Add('  Where M_kind=1  ' );
QS.SQL.Add(' and ( ' + _Where + ' )' );
```

The opening row therefore sums **both** `M_Kind = 1` detail lines **and** `M_Kind = 2` journal-summary
lines. Whenever a Rooznameh has been generated over the prior period, the opening balance of the
consolidated ledger is inflated by the summarised amount — for accounts at Kol level, roughly doubled.
The movement rows are unaffected, so the running balance is uniformly wrong by a constant. This is a
genuine numeric defect and must be fixed, not ported: the opening leg needs `M_kind = 1 and`.

Also note `Coid1.KeyValue = 0` ("all fiscal periods") emits `Where  ( <_Where> )` in the opening leg —
syntactically valid only because `_Where` is non-empty.

Print header (`:186-191`): `T1` ← `dm.RegName`, `T2` ← `'مشاهده دفتر معین تجمیعی  ' + Trim(Coid1.Text)`,
`T3` ← `M1.Text`, `T4` ← `M2.Text`, `T6` ← the date/page line. `T5` assignment is **commented out**
(`:190`). No `_Total` signature block is set.

`B_ExitClick` (`:202-205`) and `B_CloseClick` (`:197-200`) are duplicate close handlers.
**Writes: none.**

**Merge verdict (`DMoein` vs `TMoein`):** they are the same report with two different account-selection
strategies — a four-segment exact match versus a caller-supplied predicate. In the rebuild they are
**one endpoint** taking a structured account filter (`{kol?, moein?, tafsil1?, tafsil2?}` or a list of
account ids), not two.

---

### 3.4 دفتر تجمیعی — multi-account ledger listing (`DaftarT_U`)

**Launched from** `Mainu.pas:606-609` (`TMain.B_Report9Click` → `DaftarT_F.init`), toolbar button
`B_Report9` captioned `دفتر` (`Mainu.dfm:1013-1022`). Reachable.

A separate, older design: a **drill-in tree over `Sarfasl`** on the left (`G1`) and a flat line listing
on the right (`G2`). Not a running-balance ledger at all.

**Navigation.** A private `_State` (1..4) tracks the current level. `OpenQ1` (`:338-370`) rebuilds `Q1`
per level:

| `_State` | Query (`DaftarT_U.pas`) | `_Code` alias |
|---|---|---|
| 1 | `Select * into #R From Sarfasl Where S_Ko>0 and S_Mo=0` … `Order By S_Ko` (`:348-349`) | `S_Ko` |
| 2 | `… Where S_Ko=<tag> and S_Mo>0 and S_ta1=0` … `Order By S_Mo` (`:355-356`) | `S_Mo` |
| 3 | `… Where S_Ko=<tag> and S_Mo=<tag> and S_ta1>0 and S_ta2=0` … `Order By S_Ta1` (`:360-361`) | `S_Ta1` |
| 4 | `… and S_ta1=<tag> and S_ta2>0` … `Order By S_Ta2` (`:365-366`) | `S_Ta2` |

`G1DblClick` (`:226-270`) descends one level, refusing when `S_Child = 0` with
`این کد زیر شاخه ندارد` ("this code has no children", `:234`). `B_UpClick` (`:171-204`) ascends.
`Enter`/`Backspace` in the grid do the same (`:272-285`). The breadcrumb is three labels `S_Ko`,
`S_Mo`, `S_Ta1` whose `.Tag` carries the numeric code and which are shown/hidden as levels are entered.

**Multi-select is the point.** `B_OkClick` (`:94-169`) walks `Q1` and collects `_Code` for every row
where `G1.IsActiveSelected`, producing a comma list `W`, then builds the predicate by level:

```pascal
if _State=4 then W2 := ' Where M_Ko='+..S_Ko.Tag+' and M_Mo='+..S_Mo.Tag+' and M_Ta1='+..S_Ta1.Tag+' and M_Ta2 in ('+W+')';
if _State=3 then W2 := ' Where M_Ko='+..S_Ko.Tag+' and M_Mo='+..S_Mo.Tag+' and M_Ta1 in ('+W+')';
if _State=2 then W2 := ' Where M_Ko='+..S_Ko.Tag+' and M_Mo in ('+W+')';
if _State=1 then W2 := ' Where M_Ko in ('+W+')';
W2 := W2 + ' and M_kind=1 and M_Coid='+ inttostr(DM.Co_id) + ' and M_date>='
      + QuotedStr(D1.Farsi_Date) + ' and M_date<='+ QuotedStr( D2.Farsi_Date) ;
```
(`:140-150`), then

```pascal
Q2.SQL.Add(' IF OBJECT_ID(''tempdb..#R'') IS NOT NULL  DROP TABLE #R ');
Q2.SQL.Add(' Select * into #R From moein' + W2 );
Q2.SQL.Add(' Select *, S_Name, M_R From #R ');
Q2.SQL.Add('   Left Join Sarfasl On S_ko=M_Ko and S_Mo=M_Mo and S_Ta1=M_Ta1 and S_Ta2=M_Ta2');
Q2.SQL.Add('   Order By M_Date, M_Sanad ' );
```
(`:155-159`).

**Semantics.**

- **No opening balance and no running balance.** Just the raw lines in `[D1, D2]`, name-joined.
- Level-1 selection (`M_Ko in (…)`) matches **every** row under those Kol accounts at any depth — it is
  a subtree filter, unlike `DKolU`'s `M_Mo = 0`.
- `M_kind = 1` is present in both places it matters, so no `M_kind` double count here.
- Fiscal year is **`DM.CO_ID` only** — no year picker, no "all periods" option.
- **No `M_Tx` filter**: drafts included.
- `Select *, S_Name, M_R` after a `Left Join` on all four segments: `M_R` is selected but `Select *`
  already includes it — a duplicate-column select that works only because `#R` is the driving table.
  A **`LEFT JOIN` means unmatched accounts yield `S_Name = NULL`**, which is how orphaned codes surface.

**Validation** (`:98-137`): result set must be open and non-empty; at least one row selected
(`حداقل یک مورد را انتخاب کنید`, `:116`); `D1`/`D2` valid (`تاریخ را وارد کنید`, `:122,128`);
`D1 <= D2` (`رنج تاریخ را به درستی وارد کنید`, `:134`). Empty result →
`چیزی پیدا نشد` ("nothing found", `:164`). These are the most complete validation messages in the
reporting surface.

**Defaults** (`init:313-316`): `_State = 1`; `D1 = today` with `Farsi_day := 1` (first of the current
Jalali month); `D2 = today`.

**Print** (`Report2Click:377-399`): a single memo `T1` composed as
`DM.RegName + CRLF + <level title> + '  ازتاریخ: ' + D1 + '   تاتاریخ: ' + D2` with the breadcrumb
labels appended, where the level title is
`دفتر تجمیعی` / `دفتر تجمیعی کل` / `دفتر تجمیعی معین` / `دفتر تجمیعی تفضیل` for `_State` 1/2/3/4
(`:386-389`) — note these are **off by one level** relative to what is actually being listed
(`_State = 1` lists across Kol accounts but is titled with no level word at all).
`GridFontSizeChangingEx` (`:287-293`) has its two working lines commented out — the font spinner
persists a value to the INI and changes nothing.

**Writes: none.** `Q2BeforeDelete` aborts (`:372-375`).

---


---

[← SS3 General and subsidiary ledgers (1/3)](04-03-a-general-and-subsidiary-ledgers.md) | [Index](00-index.md) | [SS3 General and subsidiary ledgers (3/3) →](04-03-c-general-and-subsidiary-ledgers.md)
