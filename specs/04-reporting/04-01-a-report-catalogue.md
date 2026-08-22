_Part of [04-reporting](../04-reporting.md) — [index](00-index.md)_

## 1. Report catalogue

### 1.0 How to read this catalogue

"A report" here means **a `TfrxReport` component that some code path actually calls
`ShowReport` on**, plus the on-screen grid that feeds it where one exists. 42 `.dfm` files declare a
`TfrxReport`; 51 `ShowReport` call sites exist across 38 units. Several of those units are owned by
sibling documents — inventory reports by `05-inventory.md`, cheque/treasury reports by
`06-treasury.md`, party/shareholder reports by `07-parties-and-shareholders.md`. For those, this
catalogue records **identity, launcher, reachability, writes, and where the query lives**, and defers
the domain detail. Accounting reports are documented here in full.

Three cross-cutting facts, established once and not repeated per report:

- **Nobody filters `M_Tx`.** Not one report in the system restricts by voucher state except the
  6-column trial balance, which passes `@Sabt` to a stored procedure (§2.2). Every other number in
  every report includes **state-0 drafts**.
- **Almost nobody filters `M_kind`.** Where a report omits `M_kind = 1` it silently sums the
  journal-summary rows written by `MakeRooznamehU` alongside the detail lines (§3.0). Each entry below
  states which.
- **The `Report` toolbar button is gated by permission key 1122** (`Mainu.pas:929`), and inside its
  popup only `Report1` (1123), `Report2` (1124) and `Report9` (1141) have their own keys
  (`Mainu.pas:923-925`). `Report4`, `Report5`, `Report6`, `Report8` and `_Report9` get **no
  `IsEnabel` assignment at all** — once a user can open the popup, those five are unconditionally
  available. See §10.

### 1.1 Master index

| # | Persian name | English | Unit | Launcher | Reachable | Writes | Detail |
|---|---|---|---|---|---|---|---|
| R01 | `تراز آزمايشي 4 ستوني` | 4-column trial balance | `Taraz4Setooni_U` | `Mainu.pas:641-645`, menu `Report2` | yes | no | §2.1 |
| R02 | `تراز آزمایشی 6 ستونی` | 6-column trial balance | `Taraz6SetooniU` | `Mainu.pas:652-655`, menu `Report5` | yes | unknown (SP) | §2.2 |
| R03 | `نمایش دفتر کل` | general ledger | `DKolU` | `Mainu.pas:667-671`, menu `Report9` | yes | no | §3.1 |
| R04 | `نمایش دفتر معین` | subsidiary ledger | `DMoein` | `Mainu.pas:636-639`, menu `Report1`; drill-down from `CardJariU:390`, `RoyatJU:142,144` | yes | no | §3.2 |
| R05 | `مشاهده دفتر معین تجمیعی` | consolidated subsidiary ledger | `TMoein` | `CardJariU.pas:429` only | yes | no | §3.3 |
| R06 | `دفتر تجمیعی` | multi-account ledger listing | `DaftarT_U` | `Mainu.pas:606-609`, button `B_Report9` | yes | no | §3.4 |
| R07 | `وضعیت حسابهای کل` | Kol account monthly status | `KolStateU` | `Mainu.pas:647-650`, menu `Report4` (captioned `لیست کنترلی`) | yes | unknown (SP) | §3.5 |
| R08 | `فرم خلاصه اطلاعات جاری اشخاص` | party account summary | `CardJariU` | `Mainu.pas:445-448`, button `B_CardJari` (key 1131) | yes | no | §4 |
| R09 | `بدهکاران و بستانکاران` | debtors and creditors | `BedBes` | `Mainu.pas:662-665`, menu `Report8` | yes | no | §1.2 |
| R10 | `کنترل شماره اسناد` | voucher-number gap control | `Report6U` | `Mainu.pas:657-660`, menu `Report6` | yes | no | §1.3 |
| R11 | `رویت جامع اسناد معین` | comprehensive voucher view | `RoyatJU` | `Mainu.pas:440-443`, button `B_Report7` | yes | **yes — creates `temp_RJ_<uid>`** | §1.4 |
| R12 | `رویت جامع حسابداری` | comprehensive accounting view | `Report7U` | see §1.5 | **no** | **yes — creates `temp_R7_<uid>`** | §1.5 |
| R13 | `خلاصه اسناد معین` | voucher summary / Excel feed | `MoeinZipU` | `Mainu.pas:998-1001`, menu `SMoein6` (key 1128) | yes | no | §1.6 |
| R14 | `اسناد روزنامه` | journal voucher browser + print | `RooznamehViewU` | `Mainu.pas:1025-1028`, button `Asnad_rooznameh` (key 1132) | yes | **yes — edits `DMoein`/`Moein`** | §1.7 |
| R15 | `تبدیل اسناد معین به روزنامه` | subsidiary→journal conversion | `MoeinToRU` | `RooznamehViewU.pas:374` only | yes | **yes — inserts into `Moein`** | §1.8 |
| R16 | `چاپ سند معين` | voucher print (A) | `PrintMU` | `SanadViewU`/`SanadEditU` | yes | no | §1.9 |
| R17 | `چاپ سند معين` | voucher print (B) | `PrintM2U` | `SanadViewU`/`SanadEditU` | yes | no | §1.9 |
| R18 | `چاپ سند` | voucher print (C) | `PrintNu` | `SanadViewU`/`SanadEditU` | yes | no | §1.9 |
| R19 | `نمایش سند معین` | voucher viewer/editor + print | `SanadEditU` | `Mainu.pas:582-586`, menu `SMoein4` (key 1121); every ledger drill-down | yes | **yes — voucher editing** | §1.9 |
| R20 | `نمايش اسناد` | voucher list by state + print | `SanadViewU` | `Mainu.pas:460-473`, menus `SMoein1/2/3` (keys 1127/1125/1126) | yes | **yes — state changes, date/number edits** | §1.9 |
| R21 | `ليست سرفصلها` | chart-of-accounts list | `ListSarfaslu` | `Mainu.pas:572` — **commented out** | **no** | no | §1.10 |
| R22 | `سرفصلهای حسابداری` | chart-of-accounts browser + print | `SNewu` | `Mainu.pas:568-571`, menu `Sarfasl_List` (key 1101) | yes | **yes — account maintenance** | §1.10 |
| R23 | `سرفصلهای حسابداری` | superseded CoA browser | `S_KolU` | none | **no — dead** | — | §1.13 |
| R24 | `لیست فاکتورها جهت صدور سند` | invoices awaiting voucher issue | `SodoorSanadU` | `Mainu.pas:978-981`, button `B_Anbar` | yes | **yes — issues vouchers** | `05-inventory.md` |
| R25 | `ذخیره سند در فایل اکسل` | Darāyi Excel voucher export | `ToExcelDaraeiU` | `Mainu.pas:417-420`, menu `EBC` | yes | no | §7 |
| R26 | `ليست فاکتورهاي انبار` | warehouse invoice list | `AnbarListU` | `Mainu.pas:556-559`, button `Anbar_FactorList` | yes | see `05` | `05-inventory.md` |
| R27 | `گزارش عملیات انبار پسته` | pistachio warehouse operations | `AnbarReportU` | `Mainu.pas:983-986`, button `B_AnbarReport` | yes | see `05` | `05-inventory.md` |
| R28 | `کارت جنسي انبار` | warehouse item card | `AnbarCardJensiU` | `Mainu.pas:551-554`, `AR_Jensi` (key 1412) | yes | see `05` | `05-inventory.md` |
| R29 | `گزارش ورود و خروج انبار` | warehouse in/out | `Anbar_Amalkard` | `Mainu.pas:561-564`, `Anbar_Report` (key 1409) | yes | **yes — unconditional `UPDATE`, see §1.12** | `05-inventory.md` |
| R30 | `گزارش عملکرد انبار` | warehouse performance | `Anbar_MandehU` | `Mainu.pas:546-549`, `AR_Amalkard` (key 1413) | yes | see `05` | `05-inventory.md` |
| R31 | `گزارش ورود و خروج انبار` | warehouse in/out (purchase) | `AnbarReportKharidU` | `Mainu.pas:564` — **commented out** | **no** | see `05` | §1.13 |
| R32 | `چاپ فرم تولید کالا` | production form print | `Print_Anbar15` | `AnbarFactorU` | yes | no | `05-inventory.md` |
| R33 | `چاپ فاکتور` | invoice print | `Print_Anbar16` | `AnbarFactorU` | yes | no | `05-inventory.md` |
| R34 | `چاپ فاکتور` | invoice print | `FactorPrintU` | `AnbarFactorU` / `AnbarListU` | yes | no | `05-inventory.md` |
| R35 | `چاپ فاکتور رسمی` | official invoice print | `Factorprint2U` | `AnbarListU`, `POP_AnbarReport.AR_Chap2` | see `05` | no | `05-inventory.md` |
| R36 | `چاپ فاکتور` | invoice print v3 | `FactorPrint3U` | `Mainu.pas:883` (`FactorPrint3.init(402001)` — a hard-coded invoice number in a debug button) | see §1.13 | no | `05-inventory.md` |
| R37 | `لیست قبضهای باسکول و خرید پسته` | weighbridge/purchase receipts | `FactorPesteh_U` | `Mainu.pas:513-516`, button `B_Kharid` | yes | **yes — writes `Moein`** | `05-inventory.md` |
| R38 | `تنظیمات بانک` (cheque list) | issued-cheque list | `CheckListU` | `Mainu.pas:611-614`, `B_SodoorCheck` (key 2110) | yes | see `06` | `06-treasury.md` |
| R39 | `تنظیمات بانک` (received) | received-cheque list | `CheckListDU` | `Mainu.pas:524-527`, `B_DaryaftCheck` (key 2101) | yes | see `06` | `06-treasury.md` |
| R40 | `صدور چک` | cheque issue + print | `CheckEditU` | `CheckListU` | yes | **yes** | `06-treasury.md` |
| R41 | `لیست واریزیها` | deposit list | `FishListD` | `Mainu.pas:621-624`, `B_Variz` (key 2115) | yes | see `06` | `06-treasury.md` |
| R42 | (no caption) petty cash | petty-cash voucher + print | `TankhahEdit` | `TankhahList` ← `Mainu.pas:616-619` (key 2120) | yes | **yes** | `06-treasury.md` |
| R43 | `ورود اطلاعات انس گذاري` | assay data entry | `Lab` | none | **no — dead** | — | §1.13 |
| R44 | — | Excel export (legacy) | `ToExcelU` | creation commented out in `arzi.dpr` | **no — dead** | — | §1.13 |

### 1.2 R09 — `بدهکاران و بستانکاران` / debtors and creditors (`BedBes`)

**Purpose.** One row per *party* (`Jari` — membership/card number), showing opening balance, period
debit turnover, period credit turnover and closing balance, filtered to debtors or creditors and to an
amount window. It is the party-level counterpart of the trial balance.

**Launched from** `Mainu.pas:662-665` (`TMain.Report8Click` → `BedBesF.init`), menu item `Report8`,
caption `بدهکاران و بستانکاران` (`Mainu.dfm:10759-10761`). Reachable. **No permission key** — `Report8`
is absent from the `Reload` gating block (`Mainu.pas:909-955`).

**Form caption:** `BedBes.dfm:5`. Two filter panels plus a grid plus one report.

#### Parameter form

| Control | Type | Persian caption | Default (`init`, `:138-161`) | Validation | Passed as |
|---|---|---|---|---|---|
| `COID: TDBLookupComboBox` | fiscal year | (none) | `DM.CO_ID` (`:153`) | none | `@Coid` |
| `D1: TFullDate` | from-date | `از تاریخ :` (`.dfm:63`) | today, then `Farsi_day := 1` (`:158-159`) | `Farsi_Valid`, message `تاریخ را وارد کنید` (`:78-83`) | `@D1` |
| `D2: TFullDate` | to-date | `تا تاریخ :` | today (`:160`) | `Farsi_Valid` (`:84-89`); `D1 > D2` → `رنج تاریخ را وارد کنید`, `Exit` (`:90-95`) | `@D2` |
| `BedBes: TsComboBox` | enum ×2 | `لیست بدهکاران` / `لیست بستانکاران` (`.dfm:146-148`), `csDropDownList` | **`ItemIndex := 1`** = creditors (`:155`) — the `.dfm` design-time default is 0 | none | `@BedBes := ItemIndex + 1` (`:105`) |
| `GType: TsComboBox` | enum ×3 | `همه موارد` / `با گردش` / `بدون گردش` (`.dfm:188-191`) | `0` = all (`:154`) | none | `@GType := ItemIndex` (`:106`) |
| `M1: TEditInt` | int, `IntSplitter = ','` | `از مبلغ :` (`.dfm:123`) | **`1000000`** (`:156`) | none | `@M1` |
| `M2: TEditInt` | int | `تا مبلغ :` (`.dfm:134`) | **`100000000`** (`:157`) | none | `@M2` |

The two hard-coded amount defaults are a trap: **out of the box the report silently hides every party
whose closing balance is below 1 000 000 or above 100 000 000 rials.** Nothing on screen says so.

`COID`'s list source `Q2` is the plain `Select * From Base Order By CO_ID` (`.dfm:1116-1117`) — **no**
"all fiscal periods" synthetic row.

Every filter control has `OnChange`/`OnCloseUp = M1Change` (`:164-168`), which closes `Q1` and resets
the grid filter — so changing anything discards the result and forces a re-press of `تایید`.

#### Exact SQL (verbatim, `BedBes.dfm:444-503`; parameters bound at `BedBes.pas:100-106`)

```sql
Declare @COID int Set @COid=:Coid
Declare @D1 varchar(10) Set @D1= :D1
Declare @D2 varchar(10) Set @D2= :D2
Declare @GType int Set @GType= :GType
Declare @BedBes int Set @BedBes= :BedBes

Declare @M1 bigint Set @M1 = :M1
Declare @M2 bigint Set @M2 = :M2

if OBJECT_ID('Tempdb..#R') is not null Drop Table #R
if OBJECT_ID('Tempdb..#P') is not null Drop Table #P

Select 0 as Jari, M_Ko, M_Mo, M_Ta1, M_Ta2
      , Sum( Case When M_date<=@D1 then M_Bes-M_Bed else 0 end ) as Rem1
      , Sum( Case When M_date<=@D2 and M_Date>@D1 then M_Bed else 0 end ) as GBed
      , Sum( Case When M_date<=@D2 and M_Date>@D1 then M_Bes else 0 end ) as GBeS
      , Sum( Case When M_date<=@D2 then M_Bes-M_Bed else 0 end ) as Rem2
      , min(sarfasl.S_Name) as S_Name
into #R
From moein
left Join Sarfasl On S_ko=M_ko and S_Mo=M_Mo and S_Ta1=M_Ta1 and S_Ta2=M_Ta2
where M_coid=@Coid
Group By M_Ko, M_Mo, M_Ta1, M_Ta2

Update #R Set Jari=M_ta1 Where Jari=0 and Exists( Select * From SahamdarConfig Where M_ko=SC_K and M_Mo=SC_M and SC_T=0  and SC_Rem=1 )
Update #R Set Jari=M_ta2 Where Jari=0 and Exists( Select * From SahamdarConfig Where M_ko=SC_K and M_Mo=SC_M and SC_T=M_Ta1 and SC_Rem=1 )
Delete #R Where jari=0

Select Jari , min(S_Name) as S_Name , Sum(Rem1) As Rem1 , Sum(GBed) As GBed , Sum(GBes) As GBes , Sum(Rem2) As Rem2
   into #P From #R Group By Jari

update #P set Rem2=-Rem2 where @BedBes=1
Delete #P Where ( Rem2<@M1 or Rem2>@M2)
update #P set Rem2=-Rem2 where @BedBes=1

-- Gtype=1 گردش   Gtype=2 بدون گردش
Delete #P Where GBed=0 and GBes=0 and @Gtype=1
Delete #P Where (GBed>0 or GBes>0) and @Gtype=2

Select * From #P order by jari
```

#### Semantics, and the boundary discrepancy

- `Rem1` (`اول دوره`, opening) uses **`M_date <= @D1`** — *inclusive*. Turnover uses
  **`M_Date > @D1 and M_date <= @D2`** — *exclusive lower*. The pair is internally consistent, but it
  is **shifted one day relative to every ledger in §3**, which splits at `< @D1` / `>= @D1`. Running
  the ledger and this report with the same `D1` puts the entries dated exactly `D1` on opposite sides
  of the opening line. This is a genuine, silent reconciliation break — see §9 and §10.
- `Rem2` (`مانده نهایی`, closing) is `Σ(credit − debit)` for `M_date <= @D2` — **credit-positive**,
  matching the ledgers' `Rem`. Note `Rem1 + GBes − GBed = Rem2` holds by construction.
- **Party attribution** is the two `Update #R Set Jari=…` statements: an account is attributed to the
  party whose card number sits in Tafsil1 (when the `SahamdarConfig` template has `SC_T = 0`) or in
  Tafsil2 (when the template has `SC_T = M_Ta1`). Only templates with `SC_Rem = 1` count — the same
  subset as the Card Jari "final balance" (§4.3), and a *different* subset from the Card Jari grid.
  Rows that map to no party are deleted.
- **Debtor/creditor selection is a sign flip around the amount filter**: `@BedBes = 1`
  (`لیست بدهکاران`) negates `Rem2`, applies `@M1..@M2`, then negates back. So the amount window is
  always expressed in the *natural* direction of the chosen list. `@BedBes = 2` leaves `Rem2` alone,
  i.e. the window applies to the credit-positive value directly.
- **`@GType`**: `1` (`با گردش`) drops parties with no movement; `2` (`بدون گردش`) drops parties *with*
  movement; `0` keeps all.
- **No `M_kind` filter** — journal-summary rows are summed in. They sit at `M_Mo = 0`, so they are
  attributed only if some `SahamdarConfig` template has `SC_M = 0`, but they *are* in `#R`.
- **No `M_Tx` filter** — drafts included.
- `left Join Sarfasl` on all four segments with `min(S_Name)`: an account missing from `Sarfasl`
  yields `NULL` name and still appears.

#### Output columns

Grid `G1` (`.dfm:330-392`), persistent fields at `.dfm:506-528`:

| # | Field | Persian caption | English | Format |
|---|---|---|---|---|
| 1 | `Jari` | `جاری` (`.dfm:338`) | `party_card_no` | integer |
| 2 | `S_Name` | `نام` (`.dfm:350`) | `account_name` | string |
| 3 | `Rem1` | `اول دوره` (`.dfm:364`) | `opening_balance` | `'#,###'`, signed, zero blank |
| 4 | `GBed` | `گردش بدهکار` (`.dfm:372`) | `period_debit` | `'#,###'` |
| 5 | `GBes` | `گردش بستانکار` (`.dfm:380`) | `period_credit` | `'#,###'` |
| 6 | `Rem2` | `مانده نهایی` (`.dfm:388`) | `closing_balance` | `'#,###'`, signed, zero blank |

Report `RP1`, A4 portrait (`PaperSize = 9`, `.dfm:569`). RTL column order in `PageHeader1`:
`ردیف` (`[Line#]`), `جاری`, `مشخصات` ("particulars" — the name column is headed differently from the
grid), `اول دوره`, `گردش بدهکار`, `گردش بستانکار`, `مانده` (`.dfm:909,886,863,998,975,952,840`).
All amount memos `DisplayFormat.FormatStr = '%2.0n'`, `HideZeros = True`.

**Grouping:** none — a single `MasterData1` (`.dfm:576`). **Sort:** `order by jari` — by card number,
not by amount, which is unexpected for a "biggest debtors" report. **Page break:** none.
**Footer1** (`.dfm:1003-1080`) carries three grand totals: `[SUM(<DB1."rem2">,MasterData1)]`,
`[SUM(<DB1."GBes">,MasterData1)]` and a third `%2.0n` memo (`GBed`). **`Rem1` has no total** — the
opening column is the only one that does not foot.

#### Print-time injection (`BedBes.pas:170-183`)

`T1` ← `Dm.RegName + CRLF + Coid.Text + CRLF + BedBes.Text + CRLF + 'از مبلغ ' + M1.Text + '   تا مبلغ ' + M2.Text`.
The date range is **not** printed on the report — a report whose numbers are period-dependent does not
say which period it covers. Uses `Rp1.FindComponent` rather than `FindObject` (`:174`), like
`KolStateU` and unlike everything else.

#### Drill-down, reachability, writes

`G1DblClick` (`:130-136`) → `CardJarif.init(Jari, COID.KeyValue)` — opens the party summary (§4) for
the row. **Writes: none**; all DML is on `tempdb..#R` / `#P`.

#### Defects

1. Opening-balance boundary is one day out relative to §3 (above).
2. Amount defaults 1 000 000 … 100 000 000 silently truncate the population.
3. No `M_kind` / `M_Tx` filter.
4. `Rem1` has no column total while the other three amount columns do.
5. Sorted by card number, so the report cannot answer "who are the largest debtors" without exporting.
6. Design-time connection string with `User ID=sa`, catalogue `RPPC`, host `PESTEH` (`.dfm:1108-1113`).
7. `@Coid` is declared `ftWideString, Size = 4, Value = '1404'` (`.dfm:403-406`) and assigned an
   integer — the same coercion pattern flagged in §4.2.

### 1.3 R10 — `کنترل شماره اسناد` / voucher-number gap control (`Report6U`)

**Purpose.** Detect missing voucher numbers in a range and show each voucher's date, totals,
description and state. A control report, not a financial one.

**Launched from** `Mainu.pas:657-660` (`TMain.Report6Click` → `Report6F.init`), menu item `Report6`,
caption `کنترل شماره اسناد` (`Mainu.dfm:10752-10754`). Reachable, **no permission key**.

**Parameter form — there isn't one on the form.** `init` (`Report6U.pas:65-75`) pops a modal
two-number dialog before the form is shown:

```pascal
n1:=0; n2:=0;
Get2No('کنترل شماره اسناد', 'کنترل شماره اسناد', 'از شماره سند', 'تا شماره سند' , N1, N2 );
if (n1=0) or (n2=0) or (n1>n2)  then exit;
```

`Get2No` lives in `Get2D.pas`. Captions: dialog title and header both `کنترل شماره اسناد`, prompts
`از شماره سند` ("from voucher number") and `تا شماره سند` ("to voucher number"). Validation is the
single guard above — zero or inverted → silent `exit`, the form never opens. There is **no date
range and no fiscal-year picker**; the year is `DM.CO_ID`.

#### Exact SQL (verbatim, `Report6U.pas:80-89`)

```pascal
Q1.SQL.Add('Select M_Sanad, Min(M_Date) As M_Date, Sum(M_Bed) As M_Bed , Sum(M_Bes) As M_Bes, Min(Article) As Article ');
Q1.SQL.Add(', States = Case min(M_TX) When 0 Then ''درحال تحریر'' ');
Q1.SQL.Add('  When 1 Then ''تایید شده''  When 2 Then ''ثبت شده'' else ''UnKnown''  End ');
Q1.SQL.Add('From Moein Where M_Sanad >= '+inttostr(N1)+' and M_Sanad <= '+inttostr(N2)+' and M_COID=' + inttostr(DM.CO_ID) );
Q1.SQL.Add(' Group By M_Sanad');
Q1.SQL.Add(' Order By M_Sanad ');
```

- **`M_TX` is finally used** — but as a *displayed* value, via `min(M_TX)` over the voucher's lines,
  mapped to `درحال تحریر` (state 0, "being drafted"), `تایید شده` (1, "confirmed"),
  `ثبت شده` (2, "posted"), `UnKnown` otherwise. `min()` means a voucher with mixed line states
  displays the **lowest** state — which is the conservative choice and worth preserving.
- **No `M_kind` filter.** A journal-summary voucher (`M_Kind = 2`) occupies a voucher number in the
  same sequence and appears here, which is correct for a gap check but means `Sum(M_Bed)` mixes the
  two universes if a number is reused.
- `Min(Article)` picks an arbitrary (lexicographically smallest) line description as the voucher
  description — `DMoein.DM_Desc` would be the real one, and is not read.

#### On-screen summary lines (`Report6U.pas:91-93`)

```
S1 = 'کنترل اسناد معین و روزنامه بر حسب شماره سند'
S2 = ' از سند شماره <N1> تا سند شماره <N2> تعداد اسناد <N2-N1+1> عدد '
S3 = ' تعداد <RecordCount> سند موجود و تعداد <N2-N1+1-RecordCount> سند موجود نمیباشد '
```
i.e. "range …, count of vouchers expected", then "… vouchers present and … vouchers not present".

#### The gap list (`sBitBtn1Click`, `:97-116`)

Loops `N1..N2` and `Q1.Locate('M_Sanad', I, …)`; missing numbers are joined with `,` and a newline
every 20 entries, then shown in a `MessageDlg` headed `لیست جا خالی اسناد` ("list of voucher gaps"),
or `چیزی پیدا نشد` ("nothing found") when there are none.

**Defect:** `var I,J:integer;` (`:98`) — **`J` is never initialised** before `inc(J)` at `:104`. The
`J mod 20` line-wrap therefore starts at a garbage offset. Cosmetic, but it is an uninitialised read.
Also, `Q1.Locate` per candidate number is O(n·m) client-side scanning where one `NOT EXISTS` query
would do.

#### Print (`sBitBtn2Click`, `:118-123`)

`T1` ← `Dm.RegName`; `T2` ← `' کنترل اسناد معین و روزنامه بر حسب شماره سند از سند شماره <N1> تا سند شماره <N2>'`.
Then `Rp1.ShowReport(True)`. Output columns follow the query: voucher number, date, debit total,
credit total, description, state.

**Writes: none.** Reachable.


---

[Index](00-index.md) | [SS1 Report catalogue (2/3) →](04-01-b-report-catalogue.md)
