_Part of [04-reporting](../04-reporting.md) — [index](00-index.md)_

### 1.4 R11 — `رویت جامع اسناد معین` / comprehensive voucher view (`RoyatJU`)

**Purpose.** A drill-down turnover-and-balance view over the whole chart of accounts for a chosen
voucher-number range, date range, or party — effectively a fifth trial balance with an interactive
hierarchy. It is the most capable read screen in the system and the only one that **materialises its
result into a permanent database table**.

**Launched from** `Mainu.pas:440-443` (`TMain.B_Report7Click` → `RoyatJF.init`), toolbar button
`B_Report7`. Reachable. Form caption `رویت جامع اسناد معین` (`RoyatJU.dfm:5`).

#### Parameter form

**Scope radio group** (`_R1.._R4`), `_R1` checked in `init` (`:189`):

| Radio | Persian caption (`.dfm`) | Effect |
|---|---|---|
| `_R1` | `همه اسناد این دوره` — "all vouchers of this period" (`:151`) | no extra predicate |
| `_R2` | `بر حسب شماره سند` — "by voucher number" (`:160`) | `and M_sanad >= _N1 and M_Sanad <= _N2` (`:305`) |
| `_R3` | `بر حسب تاریخ` — "by date" (`:169`) | `and M_date >= _D1 and M_date <= _D2` (`:308`) |
| `_R4` | `حسابهای یک جاری` — "the accounts of one party" (`:178`) | `and Exists(… #R …)` (`:313`) |

Inputs: `_N1`/`_N2` (`از سند` / `تا سند`, `.dfm:63,74`), `_D1`/`_D2` (`از تاریخ` / `تا تاریخ`,
`.dfm:41,52`), `_J1` (`شماره عضویت` — membership number, `.dfm:84`).

**Validation** (`B_NewClick:200-266`) — the most complete in the reporting surface, and each branch
re-enables the button before exiting:
`_R2` → `_N1 > 0`, `_N2 > 0`, `_N1 <= _N2`, all messaged `شماره سند را وارد کنید`;
`_R3` → `_D1.Farsi_Valid`, `_D2.Farsi_Valid`, `_D1 <= _D2`, all messaged `تاریخ را وارد کنید`;
`_R4` → `_J1 > 0`, messaged `شماره عضویت را وارد کنید`.

**Presentation radio group** (`_V1.._V4`), driving `_V1ValueChanged` (`:716-725`):

| Radio | Persian caption | Shows |
|---|---|---|
| `_V1` | `نمایش بر حسب سطح حساب` — "by account level" (`.dfm:211`) | `G1`, Kol rows only, drill-down enabled |
| `_V2` | `نمایش آخرین سطح` — "last level only" (`.dfm:222`) | `G2`, `Where IsLast=1` (`:571`) |
| `_V3` | `نمایش سطح کل و معین` — "Kol and Moein levels" (`.dfm:231`) | `G2`, `Where M_Ta1=0` (`:601`) |
| `_V4` | `نمایش تمام سطوح` — "all levels" (`.dfm:240`) | `G2`, no filter (`:616-617`) |

**Fiscal year:** `DM.CO_ID` only, no picker (`Sal := 'M_Coid='+inttostr(Dm.CO_ID)`, `:269`).

#### What it writes — the per-user result table

```pascal
DB := 'temp_RJ_'+ IntToStr(Dm.userId);            // :268
Q1.SQL.Add('if Object_ID('''+ DB+ ''') Is Not Null Drop Table '+ DB );   // :275
Q1.SQL.Add(' Create Table '+ DB+ ' (IsLast Bit,  M_Ko int, SS_Ko varchar(100), M_Mo int, SS_Mo varchar(100), '+
                'M_ta1 int, SS_Ta1 varchar(100), M_ta2 int, SS_ta2 varchar(100), M_Name Varchar(250),'+
                ' TBed Bigint, TBes Bigint, Rbed Bigint, RBes Bigint, M_L Varchar(20)  )' );   // :281-283
```

**`temp_RJ_<userId>` is a real table in the application database, not a `#temp`.** It survives the
session, it is dropped and recreated on every run, and it is named per user so concurrent users do not
collide. Consequences: the production schema accumulates one such table per user who has ever run the
report; a `DROP`/`CREATE` requires DDL rights for every reporting user; and nothing cleans them up.
This is the single clearest case of "a report that writes" in the accounting half of the system.

#### The build pipeline (`B_NewClick:268-376`, eight `Waitf.Gotonextposition` steps)

1. Drop `temp_RJ_<uid>` and `tempdb..#R`.
2. Create `temp_RJ_<uid>`.
3. If `_R4`: `Select *, 0 as SC_T2 into #R From SahamdarConfig Where SC_Rem=1`, then substitute the
   card number into `SC_T2` (when `SC_T > 0`) or `SC_T` (when `SC_T = 0`) — the same template
   expansion as §4.2, but written *inline* here instead of grouped, so duplicate templates duplicate.
4. Leaf insert:
   ```sql
   insert temp_RJ_<uid> ( IsLast, M_Ko, M_Mo, M_Ta1, M_Ta2, TBed, TBes )
   Select 1, M_Ko, M_Mo, M_Ta1, M_Ta2, Sum(M_Bed), Sum(M_Bes)
   From Moein
   Where M_kind=1 and M_Coid=<CO_ID>
     [ and M_sanad >= <N1> and M_Sanad <= <N2> ]
     [ and M_date >= '<D1>' and M_date <= '<D2>' ]
     [ and Exists( Select * From #R Where M_ko=SC_K and M_Mo=SC_M and M_Ta1=SC_T and M_Ta2=SC_T2) ]
   Group By M_Ko, M_Mo, M_Ta1, M_Ta2
   ```
   (`:296-317`). **`M_kind = 1` is present** — this report does *not* double-count.
5. Three roll-up inserts with `IsLast = 0` (`:321-347`): Moein rows summed to Kol
   (`Where S.M_Mo>0 Group By S.M_Ko`), Tafsil1 summed to Moein (`Where S.M_Ta1>0`), Tafsil2 summed to
   Tafsil1 (`Where S.M_Ta2>0`). Same interleaved-levels model as the 4-column trial balance (§2.1),
   but with an explicit `IsLast` flag instead of a level number — which is strictly better, because
   `Where IsLast=1` gives a non-double-counting total.
6. Balance split (`:351-354`):
   ```sql
   Update <DB> Set RBed = TBed-TBes Where TBed>TBes
   Update <DB> Set RBes = TBes-TBed Where TBes>TBed
   Update <DB> Set RBed = 0 Where RBed is null
   Update <DB> Set RBes = 0 Where RBes is null
   ```
   the same clamped unsigned pair as everywhere else.
7. Name resolution (`:355-365`): eight `UPDATE … Set M_Name = (Select S_name From Sarfasl …)` /
   `Set SS_Ko|SS_Mo|SS_Ta1|SS_Ta2 = …` statements, one pair per level, each overwriting `M_Name` with
   the deepest matching name. The commented-out `// M_Name+''/''+` fragments show the original intent
   was a slash-joined path.
8. `Update <DB> Set M_L = Dbo.Make_R( 1, M_Ko, M_Mo, M_ta1, M_Ta2)` (`:367`) — **`@Co` hard-coded to
   `1`**, not `DM.CO_ID`. Compare `Taraz4Setooni_U.pas:145` which passes the picker value. Two call
   sites, two different arguments to the same undocumented UDF (§2.1, §9).

#### Drill-down (`G1DblClick:128-146`)

`_State` 1..4 tracks the level in `G1`. Double-click descends (`Show_Mo` → `Show_Ta1` → `Show_Ta2`);
when already at the deepest level (`_L1 = _State` after the attempt) it opens the subsidiary ledger:

```pascal
if _R3.Checked then
   DMoeinF.init( M_Ko, M_Mo, M_Ta1, M_Ta2, DM.Co_ID, _D1.Farsi_Date, _D2.Farsi_Date )
Else
   DMoeinF.init( M_Ko, M_Mo, M_Ta1, M_Ta2, DM.Co_ID  );
```

so the date range is carried into the ledger **only when the date-range scope was chosen**. Under
`_R2` (voucher-number scope) the ledger opens on the full fiscal year and the two screens disagree —
a known confusion. `G1KeyPress` (`:148-159`) is meant to ascend a level on a key press and is marked
`// not work !!!!` by the author (`:152`); the key constant is a non-ASCII literal that did not
survive encoding, so **the up-navigation is dead** and only `_V1` re-selection resets the drill.

#### Output columns (both grids, `RoyatJU.dfm:297-349` and `:396-448`)

`کد حساب` (account code, `M_L`), `نام حساب` (account name, `M_Name`), `گردش بدهکار` (`TBed`),
`گردش بستانکار` (`TBes`), `مانده بدهکار` (`RBed`), `مانده بستانکار` (`RBes`), `سطح آخر`
("last level", `IsLast`). `G1` is the drill grid, `G2` the flat grid; identical column sets.

#### Print — three separate reports

`Report2Click` (`:378-…`) picks one of **three** `TfrxReport` components by presentation mode:

- `_V1` → `Rp1` (`:463`), with `D6` rewritten to `[DB1."M_Ko"]` / `[DB1."M_Mo"]` / `[DB1."M_Ta1"]` /
  `[DB1."M_Ta2"]` according to `_State` (`:417-421`), and a `_CName` breadcrumb.
- `_V1` alternate layout → `Rp3` (`:512`).
- `_V2` → `Rp2` (`:515-…`).

All three get **runtime column-width recalculation**: the on-screen grid's first three column widths
are read (`:392-401`), a total `CT := C1+C2+4*C3` is formed, the report's `T1` width is taken as the
page width `WT`, and `C1..C6` / `D1..D6` / `S1..S4` memo widths are assigned proportionally
(`:402-406`, `:443-460`). **The printed layout follows the user's on-screen column widths.** No other
report in the system does this. It is clever and it is a porting hazard: the rebuild's print layout
must either replicate it or explicitly drop it.

`_Total` ← `Dm.Get_paramstr(1013)` (`:407`) — the ledger signature block, not the trial-balance one.

**Reachable:** yes. **Writes:** yes — `DROP TABLE` / `CREATE TABLE` / `INSERT` / `UPDATE` against
`temp_RJ_<userId>` in the application database on every run.

### 1.5 R12 — `رویت جامع حسابداری` / comprehensive accounting view (`Report7U`)

Structurally the twin of `RoyatJU`: same `temp_R7_<uid>`-style materialisation, same drill hierarchy,
same clamped balance pair, and a date range defaulting to `Farsi_day := 1` / `Farsi_day := 31`
(`Report7U.pas:334-335`). Its `Moein` filter at `:403` and `:437` is
`Where M_Date>='<Date1>' and M_date<='<Date2>'` — **with no `M_COID` and no `M_kind` predicate at
all**, so it reads every fiscal year and both `M_kind` universes at once.

**It is unreachable.** `Report7U` is listed in `Mainu.pas`'s `uses` clause (`:284`) but grep finds no
`Report7F.init` or equivalent call anywhere in the repository. There is no menu item and no button
bound to it. Treat as dead-but-instructive: it is the only report that would have crossed fiscal
years by accident, and it must not be revived without adding the two missing predicates. Full analysis
is not warranted; see §1.13.

### 1.6 R13 — `خلاصه اسناد معین` / voucher summary and Excel feed (`MoeinZipU`)

**Launched from** `Mainu.pas:998-1001` (`TMain.SMoein6Click` → `MoeinZip.init`), menu item `SMoein6`,
caption `خلاصه اسناد معین` (`Mainu.dfm:10656-10659`), permission key **1128** (`Mainu.pas:920`).
Reachable.

**Scope switch.** Two radio buttons select the filter axis, and every query in the unit branches on
them (`MoeinZipU.pas:257-258`, `:648-649`, `:671-672`):

```pascal
if _RS.Checked then _W:= 'From Moein Where M_kind=1 and M_Sanad>='+_S1.Inttext+' and M_sanad<='+_S2.Inttext+ ' and M_coid='+ inttostr(Dm.CO_ID)
               Else _W:= 'From Moein Where M_kind=1 and M_Date>='+ QuotedStr(_D1.Farsi_Date) +' and M_date<='+ QuotedStr(_D2.Farsi_Date) + ' and M_coid='+ inttostr(Dm.CO_ID) ;
```

`_RS` = by voucher-number range (`_S1`, `_S2`); `_RD` = by date range (`_D1`, `_D2`).
**`M_kind = 1` is present in every branch** — this unit does not double-count. `M_Tx` is not filtered.
Fiscal year is `DM.CO_ID`, no picker.

Four parameterised report queries share the same shape, split by side (`:590`, `:600`, `:648-649`,
`:671-672`):

```sql
Where M_kind=1 and (M_Date>=@D1 and M_Date<=@D2) and M_bed>0
Where M_kind=1 and (M_Date>=@D1 and M_Date<=@D2) and M_bes>0
Where M_kind=1 and (M_Sanad>=@S1  and M_Sanad <=@S2) and M_Bed>0
Where M_kind=1 and (M_Sanad>=@S1  and M_Sanad <=@S2) and M_Bes>0
```

i.e. the summary is produced **separately for the debit side and the credit side**, which is why the
unit carries four queries rather than one. Amounts of zero on a side are excluded outright
(`M_bed>0` / `M_bes>0`), so a line that is zero on both sides never appears.

It is also the Excel path for subsidiary vouchers — see §7. **Writes: none.**

### 1.7 R14 — `اسناد روزنامه` / journal voucher browser and print (`RooznamehViewU`)

**Launched from** `Mainu.pas:1025-1028` (`TMain.Asnad_rooznamehClick` → `RooznamehView.init`), toolbar
button `Asnad_rooznameh` captioned `اسناد روزنامه` (`Mainu.dfm:327-333,466`), permission key **1132**
(`Mainu.pas:927`). Reachable.

**This is a report *and* an editor, and it reads the header cache.**

```pascal
Q1.SQL.Add('  Select * From DMoein Where DM_kind=2 and DM_COID='+inttostr(dm.CO_ID)+' Order BY DM_Date ' );
```
(`RooznamehViewU.pas:139`) — one row per **journal-summary voucher**, straight out of `DMoein`,
including its cached `DM_TBed`/`DM_TBes` totals. Per the established facts those totals are
drift-prone; nothing here re-derives them from `Moein`. Sorted by `DM_Date` (string), with **no
secondary key**, so same-date journal vouchers list in arbitrary order.

**Per-button permission keys** (`:125-132`) — the only screen in the system that gates each action
individually:

| Button | Key | Persian | Action |
|---|---|---|---|
| `B_New` | 1133 | | create journal voucher |
| `B_Delete` | 1134 | | delete |
| `B_Date` | 1135 | `تغيير تاريخ سند` | change voucher date — **writes** |
| `B_No` | 1136 | | change voucher number — **writes** |
| `B_Desc` | 1137 | `تغییر نام` | change description — **writes** |
| `B_Sabt` | 1138 | | post/confirm — **writes** |
| `B_Lock` | 1139 | | lock — **writes** |
| `B_Print` | 1140 | | print |

**The print report (`B_PrintClick`, `:161-187`)** re-queries the lines for the selected voucher:

```pascal
Q2.SQL.Add('Select *, K_Name=( Select S_name from sarfasl Where S_ko=M_ko and S_Mo=0) ');
Q2.SQL.Add('  From moein');
Q2.SQL.Add('  Where M_Coid='+ _Coid+ ' and M_sanad='+ _Sanad  );
Q2.SQL.Add('  Order by Sign(M_bes), M_ko ');
```

- **No `M_kind` filter here** (`:177`) — if a subsidiary voucher shares the number, its lines print on
  the journal voucher. The header came from `DM_kind=2`; the body does not check.
- `Order by Sign(M_bes), M_ko` — **debits first** (`Sign(M_bes) = 0`), then credits, each block by Kol
  number. This is the standard voucher layout and is the only place in the system that uses it.
- Header injection: `t1` ← `DM.RegName + CRLF + DM.RegSal + CRLF + 'سند روزنامه'`;
  `B3` ← `'جمع : ' + Dm.Str2String(DM_TBed) + ' ریال'` — the total **spelled out in Persian words** by
  `Dm.Str2String`, taken from the **cached** `DM_TBed`, not from `Sum(M_Bed)`;
  `B4` ← `Dm.Get_paramstr(1011)` — a third configurable signature block (1011 for vouchers, 1013 for
  ledgers, 1014 for trial balances).

**`B_DateClick` (`:303-335`) is a report screen performing a two-table update:**

```sql
Begin Transaction
Declare @D1 Varchar(10) Set @D1='<new date>'
Declare @N int Set @N=<voucher no>
Declare @C int Set @C=<CO_ID>
Update DMoein Set DM_Date =@D1 Where DM_Sanad =@N and DM_CoID=@C
Update Moein  Set M_Date  =@D1 Where M_Sanad  =@N and M_CoID=@C
Commit
```

Note the `Moein` update has **no `M_kind` predicate**: changing a journal voucher's date silently
re-dates every subsidiary line that happens to carry the same voucher number in the same year. The
comment `// Test in range` at `:316` is followed by no range test — only an equality check against the
old value (`:317`). The identical statement pair appears in `SanadViewU.pas:404-405`.

**Writes: yes, extensively.**

### 1.8 R15 — `تبدیل اسناد معین به روزنامه` / subsidiary→journal conversion (`MoeinToRU`)

Form caption `تبدیل اسناد معین به روزنامه` (`MoeinToRU.dfm:5`). Not a report — a **bulk generator**
that reads a selection of subsidiary vouchers and inserts summarised `M_kind = 2` rows
(`MoeinToRU.pas:209`, `:214`, both `Insert moein (…)`). It is catalogued here because it is the second
implementation of what `MakeRooznamehU` does (§3.0) and because it is the source of the rows that
`DKolU` reports on. **Reachable, but only from inside the journal browser** —
`RooznamehViewU.pas:374` (`MoeinToR.init`) is the sole call site; there is no main-menu entry, so the
two generators (`MakeRooznamehU` from `Mainu.pas:596-599`, this one from the journal screen) are
reached by completely different routes and neither mentions the other.

Filter construction (`:162-165`):
```pascal
if _R2.Checked then S := '  Where M_Sanad>='+_N1.Inttext+' and M_Sanad<='+ _N2.Inttext;
if _R3.Checked then S := '  Where M_Date>='+QuotedStr(_D1.Text)+' and M_Date<='+ QuotedStr(_D2.Text);
S:= S+' and M_Coid='+ inttostr(Dm.CO_ID);
```

Three defects visible in five lines: **`_D1.Text` / `_D2.Text` instead of `.Farsi_Date`** (§5.5);
**no `M_kind` filter**, so a re-run summarises its own previous output; and if neither radio is
checked `S` stays empty and the appended `' and M_Coid=…'` produces `and M_Coid=1403` with no `Where`
— a syntax error. Pre-flight check at `:168-172` counts rows and reads `Min(M_Tx)`, so voucher state
is at least inspected before conversion. Full treatment belongs to `03-accounting-core.md`.


---

[← SS1 Report catalogue (1/3)](04-01-a-report-catalogue.md) | [Index](00-index.md) | [SS1 Report catalogue (3/3) →](04-01-c-report-catalogue.md)
