_Part of [03-accounting-core](../03-accounting-core.md) — [index](00-index.md)_

## 12. Screen-by-screen UI specification

RTL throughout (`BiDiMode = bdRightToLeft` on every form). Every form persists its position and size
to an INI file keyed by the form's `Name`, and grids persist per-column widths as `G1C<i>` and font
size as `G1FontSize`. In the rebuild, replace with per-user UI preferences stored server-side or in
`localStorage`.

### 12.1 Navigation map

Main form `Mainu.pas` / `TMain`. Two navigation surfaces: a bank of large speed buttons, and popup
menus attached to some of them.

```
Main
├── اسناد حسابداري  (Accounting vouchers)          → POP_Moein popup
│   ├── اسناد معين درحال تحرير   SMoein1  → SanadView.ViewAsnad(0)      [perm 1127]
│   ├── اسناد معين تاييد شده     SMoein2  → SanadView.ViewAsnad(1)      [perm 1125]
│   ├── اسناد معين ثبت شده       SMoein3  → SanadView.ViewAsnad(2)      [perm 1126]
│   ├── سند معين جديد            SMoein0  → SanadEditF.new              [perm 1113]
│   ├── مشاهده سند معین          SMoein4  → SanadEditF.View(0)          [perm 1121]
│   ├── انتقال اسناد به Excell   SMoein5  → (commented out — dead)
│   ├── خلاصه اسناد معین         SMoein6  → MoeinZip.init               [perm 1128]
│   ├── ایجاد دفاتر الکترونیکی دارایی  EBC → (tax e-ledger export)
│   └── کپی سند معين             SMoein7  → prompt + SanadEditF.Copy(n) [perm 1142]
├── اسناد روزنامه  (Journal vouchers)              → RooznamehView.init  [perm 1132]
├── سرفصلها  (Chart of accounts)                   → POP_Sarfasl popup
│   ├── ليست سرفصلها             Sarfasl_List → SNew.init               [perm 1101]
│   ├── ايجاد سرفصل              Sarfasl_Add  → DISABLED (Mainu.pas:909)
│   └── جاري اشخاص               Jari_List    → Sahamdar.init           [perm 1105]
├── گزارشها  (Reports)                             → report popup        [perm 1122]
│   ├── مشاهده دفتر معين          Report1  → DMoeinF.init(0,0,0,0)      [perm 1123]
│   ├── مشاهده دفتر تجمیعی        _Report9 → TajmiF.initM
│   ├── تراز آزمايشي 4 ستوني      Report2  → Taraz4Setooni.init         [perm 1124]
│   ├── تراز آزمایشی 6 ستونی      Report5  → Taraz6Setooni.init
│   ├── لیست کنترلی               Report4  → KolState.init
│   ├── کنترل شماره اسناد         Report6  → Report6F.init
│   ├── بدهکاران و بستانکاران     Report8  → BedBesF.init
│   └── دفتر کل                   Report9  → DKolF.init                 [perm 1141]
├── بستن حسابها      B_Enteghal1 → NewFinalF.init      [supervisor only]
├── انتقال مانده     B_Enteghal3 → EnteghalF.init      [supervisor only]
├── (unlabelled)     B_CloseMoein→ BastanhesabF.init
├── عملیات انبار     B_Anbar     → SodoorSanad.init
├── خلاصه کارت       B_CardJari  → CardJariF.init      [perm 1131]
└── (admin)          B_Admin     → AdminF.init         [supervisor only]
```

`SROOZ5` → `MakeRooznameh.init` exists as a menu handler (`Mainu.pas:594-597`) but its menu item is
not reachable from the visible menu tree — the legacy journal generator is effectively orphaned.
`SROOZ0` → `SanadMoein.NewSanad(2)` (`Mainu.pas:684-687`) is the only remaining entry to the legacy
voucher screen, and it opens it in journal mode.

### 12.2 `SNewu` — Chart of accounts (`TSNew`)

**Caption:** `'سرفصلهای حسابداری'` ("Accounting chart of accounts").
**Purpose:** browse, create, rename, renumber, delete and lock accounts, one level at a time.

**Layout:**
- Top panel: three breadcrumb `TStaticText` labels, shown as you drill down —
  `ST1` = `'کد کل : <code> <name>'` ("General code:"), `ST2` = `'کد معين : …'` ("Subsidiary code:"),
  `St3` = `'کد تفضيل : …'` ("Analytic code:"). Set at `SNewu.pas:203`, `:219`, `:236`; cleared on the
  way back up (`:378`, `:387`, `:396`).
- Toolbar (speed buttons), all with tooltips:

| Button | Caption | Hint (Persian) | English hint | Handler |
|---|---|---|---|---|
| `B_Add` | `+ جدید` | `+ افزودن کد جدید` | "+ add a new code" | `B_AddClick` (§2.1) |
| `B_DownLevel` | *(icon)* | `Enter ورود به زیر شاخه` | "Enter — go into the sub-branch" | `B_DownLevelClick` |
| `B_UpLevel` | *(icon)* | `ESC برگشت به شاخه بالاتر` | "ESC — return to the parent branch" | `B_UpLevelClick` |
| `B_EditName` | `نام` | `اصلاح نام کد` | "edit the code name" | `B_EditNameClick` (§2.2) |
| `B_EditCode` | `کد` | `اصلاح کد` | "edit the code" | `B_EditCodeClick` (§2.3) |
| `B_DeleteCode` | `حذف` | `حذف کد` | "delete the code" | `B_DeleteCodeClick` (§2.4) |
| `B_Lock` | *(icon)* | *(mislabelled `چاپ`)* | lock/unlock | `B_LockClick` (§2.5) — **admin only** |
| `S_Print` | *(icon)* | `چاپ` | "print" | FastReport, passes the three breadcrumbs as `L1`/`L2`/`L3` |
| `B_Close` | *(icon)* | *(mislabelled `چاپ`)* | "close" | `Close` |
| `GridFontSize` | spinner | — | grid font size | persisted |

- Bottom: `BitBtn4` = `'اطلاعات تکمیلی'` ("supplementary information") → `Sarfasl_Takmil` (§2.6);
  `BitBtn6` = `'برگشت'` ("back", no handler wired).

**Grid `G1` columns** (`SNewu.dfm:483-511`):

| Field | Persian title | English | Alignment | Width |
|---|---|---|---|---|
| `Code` | کد حساب | Account code | centre | 120 |
| `S_Name` | نام کامل | Full name | default | 320 |
| `SNO` | زیر شاخه | Sub-branches (child count) | centre | default |
| `S_Lock` | `?` | Lock (padlock icon) | centre | 40 |

Column widths are re-proportioned on resize to 8% / 79% / 8% / 5% of the client width, with a minimum
form width of 700 px (`SNewu.pas:671-682`).

**Level queries** (`Code` is the level's own code component, zero-padded for display at levels 2–3):

```sql
-- level 1, SNewu.pas:485-489
 Select S.* , S.S_Ko As Code
 ,SNO = S_Child
 From Sarfasl As S
 Where S_ko>0 And S_mo=0
 Order By S_Ko

-- level 2, SNewu.pas:499-506
 If OBJECT_ID('tempdb..#R') IS NOT NULL Drop Table #R
 Select S.* , Code=  Cast(S.S_Mo As varchar(9) )
 ,SNO = S_Child
 Into #R From Sarfasl As S
 Where S_ko=<kol>  And S_mo>0 And S_ta1=0
 Update #R Set Code= '0'+ Code Where Len(Code) < 3
 Update #R Set Code= '0'+ Code Where Len(Code) < 3
 Select * From #R Order By S_Mo

-- level 3, SNewu.pas:517-525
 If OBJECT_ID('tempdb..#R') IS NOT NULL Drop Table #R
 Select S.* , Code=  Cast(S.S_Ta1 As varchar(9) )
 ,SNO = S_Child
 Into #R From Sarfasl As S
 Where S_ko=<kol>  And S_mo=<moein> And S_ta1>0  and S_ta2=0
 Update #R Set Code= '0'+ Code Where Len(Code) < 4
 Update #R Set Code= '0'+ Code Where Len(Code) < 4
 Update #R Set Code= '0'+ Code Where Len(Code) < 4
 Select * From #R Order By S_Mo

-- level 4, SNewu.pas:536-540
 Select S.* , S.S_Ta2 As Code
 ,SNO = 0
 From Sarfasl As S
 Where S_ko=<kol>  And S_mo=<moein> And S_ta1=<taf1>  and S_ta2>0
 Order By S_Ta1
```

Note: the zero-padding widths (3 for Moein, 4 for Tafsil-1) are **hard-coded here**, contradicting the
configurable `Base.No_Mo` / `Base.No_Ta1` (§1.3). Levels 1 and 4 are not padded at all. Levels 2 and 3
sort by the numeric column, level 4 sorts by `S_Ta1` (a constant within the level — effectively
unsorted). **Normalise all of this in the rebuild.**

**Navigation rules** (`SNewu.pas:197-242`):
- Level 1 → 2: always allowed.
- Level 2 → 3 and 3 → 4: **blocked if the node has postings**
  (`' بر روی این کد سند صادر شده است و قابلیت افزودن زیر شاخه ندارد '` = "A voucher has been issued
  against this code and it cannot have sub-branches") — `SNewu.pas:214`, `:230`.
- Level 4 has no further level; `B_DownLevelClick` falls through and does nothing.

### 12.3 `SanadViewU` — Voucher browser (`TSanadView`)

**Caption:** set per mode at `SanadViewU.pas:126-128`:
`'نمايش اسناد معين در حال تحرير'` / `'نمايش اسناد معين تاييد شده'` / `'نمايش اسناد معين ثبت شده'`
("Display of subsidiary vouchers in preparation / approved / posted").

**Entry:** `SanadView.ViewAsnad(TX)` with `TX ∈ {0,1,2}`. The window is always maximised to the work
area and its system menu is stripped (`Dm.Disable_Key`, `SanadViewU.pas:543-546`).

**List query** (`SanadViewU.pas:723-725`):

```sql
Select * From DMoein
  Where DM_Kind=1 and DM_Tx=<TX> and DM_Coid=<CO_ID>
  Order By DM_Sanad
```

Positioned on the **last** row (`Q1.Last`, `SanadViewU.pas:727`).

**Grid `G1` columns** (`SanadViewU.dfm:530-591`):

| Field | Persian title | English | Alignment | Notes |
|---|---|---|---|---|
| `DM_Lock` | `?` | Lock | — | owner-drawn padlock icon (`SanadViewU.pas:548-579`) |
| `DM_Atf` | عطف | Folio / cross-reference | centre | never written (§3.2) |
| `DM_Sanad` | *(field default)* | Voucher number | centre | |
| `DM_Date` | *(field default)* | Date | centre | |
| `DM_TBed` | *(field default)* | Total debit | left | `DisplayFormat = ###,###` |
| `DM_TBes` | *(field default)* | Total credit | left | `DisplayFormat = ###,###` |
| `DM_Count` | *(field default)* | Line count | centre | |
| `DM_TX` | *(field default)* | State | centre | rendered via `Q1DM_TXGetText` (§3.6) |
| `DM_Desc` | *(field default)* | Narration | default | |

**Buttons** (right-hand panel `PR`), with visibility driven by permission **and** current state
(`SanadViewU.pas:744-757`):

| Button | Caption | English | Permission | Visible when |
|---|---|---|---|---|
| `B_New` | سند جديد | New voucher | 1113 | `TX = 0` |
| `B_Edit` | اصلاح سند | Edit voucher | 1114 | `TX = 0` |
| `B_Delete` | حذف سند | Delete voucher | 1115 | `TX = 0` |
| `B_ChangeDate` | تغيير تاريخ | Change date | 1119 | `TX = 0` |
| `B_ChangeNo` | تغيير شماره | Change number | 1120 | `TX = 0` |
| `B_TX01` | تاييد سند | Approve voucher | 1116 | `TX = 0` |
| `B_TX10` | برگشت به تحرير | Return to draft | 1118 | `TX = 1` |
| `B_TX12` | ثبت دائم سند | Post permanently | 1117 | `TX = 1` |
| `B_TX21` | برگشت به تایید | Return to approved | 1145 | `TX = 2` |
| `B_ViewPrint` | مشاهده و چاپ | View and print | 1121 | always |
| `B_Copy` | کپی سند | Copy voucher | 1142 | always |
| `B_Merge` | ادغام | Merge | 1143 | always |
| `B_Lock` | قفل سند | Lock voucher | 1144 | always |
| `B_Exit` | خروج | Exit | — | always |

`B_Lock` opens `PopupMenu1` positioned under the button (`SanadViewU.pas:692-697`), with items
`'قفل سند'` ("lock voucher"), a separator, and `'برداشتن قفل'` ("remove lock").

**Change voucher number** — `B_ChangeNoClick` (`SanadViewU.pas:319-372`):
Prompt `GetNo('تغيير شماره سند', 'شماره سند جديد', S1)` ("change voucher number" / "new voucher
number"), confirmation `GetYes('information','تغيير شماره سند','انجام شود ؟')` ("… / shall it be
done?"), duplicate check `'  شماره سند تکراري است   '` ("the voucher number is duplicate"), then:

```sql
 Begin Transaction
 Declare @OLD  int Set @OLD=<old>
 Declare @New  int Set @New= <new>
 Declare @COID int Set @COID= <CO_ID>
 Update DMoein        Set DM_Sanad=@New  Where DM_Sanad=@OLD  and DM_CoID=@COID
 Update Moein         Set M_Sanad=@New   Where M_Sanad=@OLD   and M_CoID=@COID
 Update Anbar_Factor  Set AF_Sanad=@New  Where AF_Sanad=@OLD  and AF_CoID=@COID
 Update DFish         Set S_Sanad=@New   Where S_Sanad=@OLD   and S_CoID=@COID
 Update DCheck        Set S_Sanad=@New   Where S_Sanad=@OLD   and S_CoID=@COID
 Update DCheck2       Set S_Sanad=@New   Where S_Sanad=@OLD   and S_CoID=@COID
 Update CheckMaster   Set CM_Sanad=@New  Where CM_Sanad=@OLD  and CM_CoID=@COID
 Update TankhahMaster Set TM_Sanad=@New  Where TM_Sanad=@OLD  and TM_CoID=@COID
 Update <Anbar_DB>.FactorMaster Set FM_SanadNo=@New Where FM_Sanadno=@OLD and FM_CoID=@COID
Commit
```

The last statement is emitted only when the inventory database exists (`SanadViewU.pas:362`).
**Eight tables + the inventory database.** This is the authoritative list of everything that
references a voucher number; use it as the FK map for the rebuild.

**Change voucher date** — `B_ChangeDateClick` (`SanadViewU.pas:374-426`):
Prompt `GetDate('تغيير تاريخ سند', 'تاريخ جديد', D)` ("change voucher date" / "new date"),
confirmation `'تغيير تاريخ سند'` / `'انجام شود ؟'`, then:

```sql
 Begin Transaction
 Declare @D1 Varchar(10) Set @D1='<new date>'
 Declare @N int Set @N=<voucher>
 Declare @C int Set @C=<CO_ID>
 Update DMoein Set DM_Date =@D1 Where DM_Sanad =@N and DM_CoID=@C
 Update Moein Set M_Date =@D1 Where M_Sanad =@N and M_CoID=@C
 Update Dfish Set S_Date=@D1 where S_Sanad =@N and S_CoID=@C
 Update DCheck Set S_Date=@D1 where S_Sanad =@N and S_CoID=@C
 Update DCheck2 Set S_Date=@D1 where S_Sanad =@N and S_CoID=@C
 Update CheckMaster Set CM_Date=@D1 Where CM_Sanad=@N and CM_Coid=@C
 Update TankhahMaster Set TM_Date=@D1 Where TM_Sanad=@N and TM_Coid=@C
 Update Anbar_Factor Set AF_Date=@D1 Where AF_Sanad=@N and AF_CoID=@C
 Update Anbar_FactorD Set AFD_Date=@D1 Where AFD_Factor in (
   Select isnull(AF_Factor,-1) From Anbar_Factor Where AF_COID=@C and AF_Sanad=@N ) and AFD_CoID=@C
 Update <Anbar_DB>.FactorMaster Set FM_SanadDate=@D1  Where FM_Sanadno=@N  and FM_CoID=@C
 Commit
```

**Nine tables + the inventory database.** Note it **does not validate** that the new date is inside
the fiscal year — an omission.

**Copy voucher** — `B_CopyClick` (`SanadViewU.pas:654-685`). Eligibility probe:

```sql
 Select Max(M_link) as M_link , Max(M_Id) as M_ID From moein
    Where M_Sanad=<n> and M_coid=<CO_ID>
```

If `M_link > 0` → `'      فقط اسناد صادره دستی قابل کپی شدن هستند      '` ("Only manually issued
vouchers can be copied"). Otherwise opens `SanadEditF.Copy(S1)` (§12.4).

---

_Prev: [03-11-index-of-all-sql-in-the-accounting-core](03-11-index-of-all-sql-in-the-accounting-core.md) | Next: [03-12-b-screen-by-screen-ui-specification](03-12-b-screen-by-screen-ui-specification.md)_
