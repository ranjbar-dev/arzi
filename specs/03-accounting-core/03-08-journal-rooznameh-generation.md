_Part of [03-accounting-core](../03-accounting-core.md) — [index](00-index.md)_

## 8. Journal (Rooznameh) generation

A **Rooznameh voucher** is a summary document that rolls a range of Moein vouchers up to the
**general-ledger (Kol) level**. It is stored in the same two tables, discriminated by
`DM_Kind = 2` / `M_Kind = 2`.

Two generators exist. **`MoeinToRU.pas` is the live one**; `MakeRooznamehU.pas` is legacy.

### 8.1 `MoeinToRU.pas` — the live generator

Opened from `RooznamehViewU.B_NewClick`, permission `1133` (`'ثبت سند روزنامه'` = "record journal
voucher").

**Input** (defaults set at `MoeinToRU.pas:239-257`):

| Control | Meaning |
|---|---|
| `_R2` / `_R3` (radio) | Select source range **by voucher number** (`_R2`) or **by date** (`_R3`). Default `_R3`. |
| `_N1`, `_N2` | Voucher-number range (visible when `_R2`) |
| `_D1`, `_D2` | Jalali date range (visible when `_R3`) |
| `_Sanad` | Target journal voucher number |
| `_Date` | Journal voucher date |
| `_Desc` | Journal voucher narration |
| `sSpeedButton1` | Fetch the next free voucher number |

The radio choice swaps which pair of controls is visible, at the same screen position
(`MoeinToRU.pas:271-286`).

Next-number button (`MoeinToRU.pas:259-269`):

```sql
  Select isnull(Max(DM_Sanad) , 0)+1 As N From DMoein Where DM_Coid=<CO_ID>
```

**Validations, in order** (`MoeinToRU.pas:61-186`):

| # | Condition | Persian message | English | Cite |
|---|---|---|---|---|
| 1 | `_R2` and `_N1 <= 0` | `'   شماره ابتدایی اسناد را وارد کنید   '` | "Enter the starting voucher number" | `MoeinToRU.pas:69` |
| 2 | `_R2` and `_N2 <= 0` | `'   شماره انتهای اسناد را وارد کنید   '` | "Enter the ending voucher number" | `MoeinToRU.pas:75` |
| 3 | `_R2` and `_N1 > _N2` | `'   رنج اسناد را وارد کنید   '` | "Enter the voucher range" | `MoeinToRU.pas:81` |
| 4 | `_R3` and `_D1` invalid | `'    تاریخ شروع اسناد را وارد کنید   '` | "Enter the vouchers' start date" | `MoeinToRU.pas:90` |
| 5 | `_R3` and `_D2` invalid | `'    تاریخ پایان اسناد را وارد کنید   '` | "Enter the vouchers' end date" | `MoeinToRU.pas:96` |
| 6 | `_R3` and `_D1 > _D2` | `'   رنج تاریخ را وارد کنید   '` | "Enter the date range" | `MoeinToRU.pas:102` |
| 7 | `_Date` invalid | `'    تاریخ سند روزنامه  را وارد کنید   '` | "Enter the journal voucher date" | `MoeinToRU.pas:110` |
| 8 | `_Date < Base.FromDate` | `Format('   تاریخ سند روزنانه نباید کمتر از %s باشد  ', [s])` | "The journal voucher date must not be earlier than %s" *(typo: روزنانه)* | `MoeinToRU.pas:123` |
| 9 | `_Date > Base.ToDate` | `Format('   تاریخ سند روزنانه نباید بیشتر از %s باشد  ', [s])` | "The journal voucher date must not be later than %s" | `MoeinToRU.pas:131` |
| 10 | `_Sanad <= 0` | `'   شماره سند روزنامه را واردکنید   '` | "Enter the journal voucher number" | `MoeinToRU.pas:139` |
| 11 | `_Sanad` already exists in `DMoein` | `'   شماره سند تکراری است   '` | "The voucher number is duplicate" | `MoeinToRU.pas:150` |
| 12 | `Length(Trim(_Desc)) <= 3` | `'   شرح سند را وارد کنید   '` | "Enter the voucher narration" | `MoeinToRU.pas:156` |
| 13 | no `Moein` rows in the range | `'   در رنج مشخص شده سندی وجود ندارد   '` | "There is no voucher in the specified range" | `MoeinToRU.pas:176` |
| 14 | `Min(M_Tx) < 2` in the range | `'   قبل از ایجاد سند باید تمامی اسناد در محدوده مشخص شده ثبت دائم شده باشند   '` | "Before creating the voucher, all vouchers in the specified range must be permanently posted" | `MoeinToRU.pas:183` |

Check 14 is the key business rule: **a journal voucher may only summarise permanently-posted
(`M_Tx = 2`) vouchers.**

Range predicate (`MoeinToRU.pas:162-165`):

```pascal
  if _R2.Checked then S := '  Where M_Sanad>='+_N1.Inttext+' and M_Sanad<='+ _N2.Inttext;
  if _R3.Checked then S := '  Where M_Date>='+QuotedStr(_D1.Text)+' and M_Date<='+ QuotedStr(_D2.Text);
  S:= S+' and M_Coid='+ inttostr(Dm.CO_ID);
```

**Note:** the predicate does **not** filter on `M_Kind`, so a previously-generated journal voucher
inside the range would be summarised again. See §14.

**Generation** (`MoeinToRU.pas:190-221`):

```sql
-- verbatim
 if OBJECT_ID('TempDB..#R') is not null Drop Table #R

 Declare @Sanad int Set @Sanad=<targetNo>
 Declare @COID int Set @COID=<CO_ID>
 Declare @Date varchar(10) Set @Date='<jalali>'
 Declare @Desc varchar(200) Set @Desc='<narration>'
 Declare @User int Set @User=<userId>
 Begin Transaction
Select Sum(M_Bed) As M_Bed , Sum(M_Bes) As M_Bes,M_ko, 0 as M_Code
  Into #R
  From Moein
  <range predicate>
  Group By M_Ko
  Order By M_Ko
Update #R Set M_Code = (Select S_SSN From Sarfasl Where S_Ko=M_ko and S_Mo=0 and S_ta1=0 and S_ta2=0)

 Insert moein (M_Coid, M_Sanad, M_Date, M_bed, M_bes, M_Ted, Article, M_Tx, M_Ko, M_Mo, M_Ta1, M_Ta2,
               M_ID, M_Link, M_User, M_kind, M_Code )
  Select @COID, @Sanad, @Date, M_bed, 0, 0, @Desc, 0, M_Ko, 0,0,0,0,0, @User, 2, M_Code
  From #R
  Where #R.M_bed > 0

 Insert moein (M_Coid, M_Sanad, M_Date, M_bed, M_bes, M_Ted, Article, M_Tx, M_Ko, M_Mo, M_Ta1, M_Ta2,
               M_ID, M_Link, M_User, M_kind, M_Code )
  Select @COID, @Sanad, @Date, 0, M_Bes, 0, @Desc, 0, M_Ko, 0,0,0,0,0, @User, 2, M_Code
  From #R
  Where #R.M_bes > 0

 Commit
  Select * From #R order By M_ko
```

**What it produces:**
- One temp-table row per Kol account: total debit and total credit (**gross** turnover, not net).
- Then **all debit lines first** (ordered by `M_Ko`), **then all credit lines** (ordered by `M_Ko`).
- Each line: `M_Kind = 2`, `M_Tx = 0`, `M_Mo/M_Ta1/M_Ta2 = 0`, `M_ID = 0`, `M_Link = 0`,
  `M_Code = <Kol header account id>`, `Article = <the journal narration>` (identical on every line).
- A Kol with both debit and credit turnover produces **two lines**, one in each block.

Finally `Dm.DMoein_Make(_Sanad, _Date, _Desc, 2)` (`MoeinToRU.pas:223`) creates the header with
`DM_Kind = 2`. Success: `'  سند روزنامه صادر شد   '` ("the journal voucher was issued").

**Numbering:** the journal voucher takes a number from the **same sequence as Moein vouchers**
(`DMoein` is shared). There is no separate journal numbering series.

**Re-runnable?** Yes, arbitrarily — nothing marks the source vouchers as journalised. The only
protection is the duplicate-number check (validation 11). Overlapping ranges double-count silently.
See §14.

### 8.2 `MakeRooznamehU.pas` — the legacy generator

Reachable from menu item `SROOZ5` (`Mainu.pas:594-597`). Inputs: `S1`/`S2` (voucher range), `S3`
(target number), `D1` (date), `Desc1` (narration).

Validations (`MakeRooznamehU.pas:62-90`):

| # | Condition | Persian message | English | Cite |
|---|---|---|---|---|
| 1 | count of vouchers in range ≠ `S2-S1+1` | `'تعداد اسناد ناقص است'` ("The voucher count is incomplete") | **DEAD — the whole block is commented out** | `MakeRooznamehU.pas:73-78` |
| 2 | `S3` already exists in `Moein` | `'شماره سند کل تکراري است'` | "The general voucher number is duplicate" | `MakeRooznamehU.pas:87` |

There is **no permanent-posting requirement** here — that is why `MoeinToRU` replaced it.

Aggregation (`MakeRooznamehU.pas:94-100`):

```sql
 Select M_Ko , Sum(M_Bed) As Bed, Sum(M_bes) As Bes
 From Moein  Where M_Sanad>=<S1> and M_Sanad<=<S2>
 and M_kind=1 and M_COID=<CO_ID>
 Group By M_Ko
 Order By M_ko
```

(This one *does* filter `M_kind=1` — better than the live generator.)

Then two loops writing through the `TADOTable` API rather than SQL: debit lines first
(`MakeRooznamehU.pas:106-131`), then credit lines (`:133-159`), with `M_Kind = '2'`,
`M_Mo/Ta1/Ta2 = '0'`, `M_ID = '0'`, `M_Link = '0'`, `M_Code = '0'`, `M_TX = '0'`.

**Note `M_Code = 0`** — unlike `MoeinToRU`, this generator does not resolve the Kol account id. Lines
produced this way have no `account_id`.

Finally it opens the legacy voucher editor on the result: `SanadMoein.EditSanad( S3, 2 )`
(`MakeRooznamehU.pas:162`). It does **not** call `DMoein_Make`, so **no header row is created**.

**Overflow bug:** `Q1.FieldByName('Bed').AsInteger` (`MakeRooznamehU.pas:108`, `:136`) reads a
`bigint` sum into a 32-bit integer. Sums above 2,147,483,647 rials — a trivial amount in Iranian
accounting — raise a conversion error or truncate. Another reason this unit is superseded.

### 8.3 The journal voucher browser — `RooznamehViewU`

Opened from the main form's `Asnad_rooznameh` speed button (`Mainu.pas:1023-1026`), caption
`'اسناد روزنامه'` ("journal vouchers"), gated by permission `1132` (`Mainu.pas:925`).

List query (`RooznamehViewU.pas:135-139`):

```sql
  Select * From DMoein Where DM_kind=2 and DM_COID=<CO_ID> Order BY DM_Date
```

Ordered **by date**, not by number.

Buttons and their permissions (`RooznamehViewU.pas:125-132`):

| Button | Persian | English | Permission | Behaviour |
|---|---|---|---|---|
| `B_New` | ثبت سند روزنامه | Create journal voucher | 1133 | opens `MoeinToR.init` |
| `B_Delete` | حذف سند روزنامه | Delete journal voucher | 1134 | see below |
| `B_Date` | تغییر تاریخ سند روزنامه | Change journal voucher date | 1135 | updates `DMoein.DM_Date` and `Moein.M_Date` in a transaction |
| `B_No` | تغییر شماره سند روزنامه | Change journal voucher number | 1136 | duplicate check, then updates `DMoein` + `Moein` |
| `B_Desc` | تغییر شرح سند روزنامه | Change journal voucher narration | 1137 | `Update DMoein Set DM_desc=… Where DM_SSN=…` |
| `B_Sabt` | تغییر وضعیت سند | Change voucher state | 1138 | **NO HANDLER — dead button** |
| `B_Lock` | قفل سند روزنامه | Lock journal voucher | 1139 | popup menu, toggles `DM_Lock` |
| `B_Print` | چاپ سند روزنامه | Print journal voucher | 1140 | FastReport |

**Delete guards** (`RooznamehViewU.B_DeleteClick`):

| # | Check | Persian | English |
|---|---|---|---|
| 1 | `DM_TX <> 0` | `'   امکان حذف سند وجود ندارد. سند در حالت تحریر نیست.   '` | "The voucher cannot be deleted. The voucher is not in draft state." |
| 2 | `DM_lock = 1` | `'   امکان حذف سند وجود ندارد سند  قفل شده است   '` | "The voucher cannot be deleted, the voucher is locked" |
| 3 | confirmation | `'    سند '+S+' حذف شود ؟   '` | "Shall voucher S be deleted?" |

```sql
 Delete moein Where M_Sanad=<n> and M_COid=<CO_ID>
 Delete Dmoein Where DM_Sanad=<n> and DM_COid=<CO_ID>
```

Success: `'   سند حذف شد   '` ("the voucher was deleted").

**Note:** the delete predicate is not scoped by `M_Kind`. Since `DM_Sanad` is unique per `DM_Coid` this
is theoretical, but it means a journal voucher and a Moein voucher can never share a number.

**Change-date and change-number here are much narrower than the Moein equivalents** — they touch only
`DMoein` and `Moein`, not the seven subsystem tables. Correct, since journal vouchers have no
subsystem links.

Change-number duplicate check (`RooznamehViewU.B_NoClick`):
`'  شماره سند تکراري است   '` ("the voucher number is duplicate").

### 8.4 Printing a journal voucher

`RooznamehViewU.B_PrintClick`:

```sql
Select *, K_Name=( Select S_name from sarfasl Where S_ko=M_ko and S_Mo=0)
  From moein
  Where M_Coid=<CO_ID> and M_sanad=<n>
  Order by Sign(M_bes), M_ko
```

`Order by Sign(M_bes)` puts debit lines (where `M_Bes = 0`, so `Sign = 0`) first, then credits. The
total is rendered in Persian words via `Dm.Str2String` (`Dmu.pas:604-635`) and appended with
`' ریال'` ("rials"). The signature block text comes from `Dm.Get_paramstr(1011)` = `'سند امضا 1'`
("voucher signature 1") — `Dmu.pas:482`.

---

_Prev: [03-07-merging-vouchers-mergesanad-pas](03-07-merging-vouchers-mergesanad-pas.md) | Next: [03-09-a-period-close-and-year-end](03-09-a-period-close-and-year-end.md)_
