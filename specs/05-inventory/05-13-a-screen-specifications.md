_Part of [05-inventory](../05-inventory.md) — [index](00-index.md)_

## 13. Screen specifications

Input for the React rebuild. Every screen is listed with its entry point, purpose, controls, grid
columns, button actions and navigation. **Dead and unreachable controls are flagged in bold.**
Screens already specified in detail elsewhere are cross-referenced rather than repeated.

### 13.0 Screen inventory and reachability

| # | Unit | Persian caption | English | Entry point | Reachable? |
|---|---|---|---|---|---|
| 1 | `AnbarTanzimU` | *(settings)* | Warehouse settings | `Mainu.pas:534-537` `Anbar_TanzimClick` | yes |
| 2 | `AnbarCalaU` | `انبار کالا` | Item list | `Mainu.pas:539-542` `Anbar_AjnasClick` | yes |
| 3 | `AnbarCalaAddU` | `مشخصات کالا` | Item editor | modal from #2 | yes |
| 4 | `AnbarCalaSelectU` | *(item search)* | Item picker | modal from #7, #9 | yes |
| 5 | `AnbarListU` | *(invoice list)* | Invoice list | `Mainu.pas:554-557` `Anbar_FactorListClick` | yes |
| 6 | `AnbarFactorU` | `فاکتور کالا` | Invoice editor | modal from #5 | yes |
| 7 | `AnbarFactorAddU` | `افزودن به فاکتور` | Invoice line editor | modal from #6 | yes |
| 8 | `Anbar_MandehU` | `گزارش عملکرد انبار` | Stock balance report | `Mainu.pas:544-547` `AR_AmalkardClick` | yes |
| 9 | `AnbarCardJensiU` | *(stock card)* | Item movement card | `Mainu.pas:549-552` `AR_JensiClick` | yes |
| 10 | `Anbar_Amalkard` | `گزارش ورود و خروج انبار` | Inbound/outbound report | `Mainu.pas:559-564` `Anbar_ReportClick` | yes |
| 11 | `AnbarReportU` | *(subsystem B activity)* | External-warehouse activity | `Mainu.pas:983` | yes |
| 12 | `AnbarReportKharidU` | *(purchase summary)* | Purchase/sale summary | `Mainu.pas:562` — **commented out** | **NO — unreachable** |
| 13 | `TasfiehFactor` | `تسویه حساب فاکتور` | Invoice settlement | from #5 and #16 | yes — §9 |
| 14 | `FactorPrintU` | *(invoice print)* | Invoice print, variant 1 | **nothing calls `FactorPrint.init`** | **NO — unreachable** |
| 15 | `Factorprint2U` | *(official invoice)* | Official invoice print | `AnbarListU.pas:289` | yes |
| 16 | `FactorPrint3U` | *(invoice print)* | Invoice print, 4 forms | `AnbarListU.pas:316` | yes |
| 17 | `SodoorSanadU` | *(post voucher)* | Subsystem-B document list / posting | `Mainu.pas:978` | yes |
| 18 | `MakeSanadU` | `صدور سند …` | Voucher preview / confirm | modal from #17 | yes — §10.2 |
| 19 | `Print_Anbar15` | *(production print)* | Production document print | `SodoorSanadU.pas:279` | yes |
| 20 | `Print_Anbar16` | *(transfer print)* | Transfer document print | `SodoorSanadU.pas:283` | yes |
| 21 | `FactorPesteh_U` | `لیست قبضهای  باسکول و خرید پسته` | Pistachio purchase list | `Mainu.pas:511-514` | yes — §8.4.1 |
| 22 | `Kharid_U` | *(pistachio purchase)* | Pistachio purchase entry | `Mainu.pas:362-365`, on a hidden panel | **NO — §8.0.1** |
| 23 | `PestehD_U` | `مشخصات پسته` | Pistachio deduction calculator | modal from #22 | **NO — §8.0.1** |
| 24 | `Kharid_BU` | `اطلاعات پایه خرید و فروش پسته` | Pistachio base accounts | `Mainu.pas:367-370`, hidden panel | **NO — §8.0.1** |
| 25 | `Lab` / `Ghabz` / `Get_Serial` | *(weighbridge/lab)* | Blind-coding a lot | **not referenced anywhere** | **NO — §8.6** |

> **Correction to the brief.** The three `FactorPrint*` units are *not* three menu items on
> `AnbarListU`. Only `Factorprint2` (`چاپ فاکتور رسمی`) and `Factorprint3` (`چاپ فاکتور`) are
> invoked; `FactorPrintU` appears in `AnbarListU.pas:138`'s `uses` clause but
> `FactorPrint.init` is called from nowhere in the repository. It is a dead unit that still
> compiles into the binary.
>
> The brief's other claim — that `Print_Anbar15` and `Print_Anbar16` are not duplicates — is
> **confirmed**: they handle different document pairs (15/25 production vs 16/26 transfer) with
> materially different logic (§3.2.3).

---

### 13.1 Warehouse settings — `AnbarTanzimU`

**Purpose:** maintain `Anbar_Config` — warehouse name, VAT rate and the six posting accounts (§1.1).

Layout: a grid `G1` over `DM.AnbarConfig` on the left, a detail panel on the right. Selecting a
grid row calls `G1ScrollData` → `ReLoad` (`AnbarTanzimU.pas:194-197`).

| Control | Persian label | Type | Behaviour |
|---|---|---|---|
| `CA_Name` | *(warehouse name)* | edit | free text |
| `CA_DMaliat` | *(VAT %)* | edit | `CA_DMaliatKeyPress` (`:199-206`) rejects everything but `0-9` and backspace — **a decimal point cannot be typed** (§7.3.1) |
| `A_Kharid` + `B_Kharid` + `A_KharidName` | `کد خرید` | account code + picker + name | typing fires `A_KharidChange` (`:114-122`) which resolves through `Taraf` and writes the resolved `S_SSN` into the edit's `.Tag`; the `?` button opens the `Taraf` picker modal (`B_KharidClick`, `:139-153`) |
| `A_BKharid` … | `کد برگشت از خرید` | same | same handlers, resolved generically by `FindComponent(Name + 'Name')` |
| `A_Foroosh` … | `کد فروش` | same | |
| `A_BForoosh` … | `کد برگشت از فروش` | same | |
| `A_Kasr` … | `کد تخفیف` | same | |
| `A_Maliat` … | `کد مالیات` | same | |

**Generic handler pattern worth copying conceptually:** one `OnChange` and one `OnClick` serve all
six account fields, dispatching on `(Sender as TsEdit).Name` and `FindComponent(Name + 'Name')`.
It is why adding a seventh account requires no new code.

| Button / menu | Caption | Action |
|---|---|---|
| `B_Save` | *(save)* | `B_SaveClick` (`:168-192`) — validates only that `CA_DMaliat` is non-empty, then writes all eight fields and shows `ذخیره انجام شد` ("save completed") |
| `B_Close` | *(close)* | `Close` |
| `P1` / `N1` | `ورود انبار جديد` prompt | Add warehouse: `GetString('ورود انبار جديد','نام انبار',50,…)` then `insert Anbar_Config (AC_ID, AC_Name) Values (Max(AC_ID)+1, …)` (`:208-225`) |
| `P1` / `N3` | same prompt text | Rename warehouse (`:227-237`) — **note the prompt still says "enter a new warehouse"** |
| **`P1` / `N2`** | — | **DEAD — declared at `AnbarTanzimU.pas:22`, no handler.** Presumably "delete warehouse". |

**Keyboard navigation:** `A_KharidKeyDown` (`:124-137`) maps Enter and ↓ to "next control" and ↑ to
"previous control" via `WM_NEXTDLGCTL`; `A_KharidKeyPress` (`:239-242`) swallows Enter so it does
not trigger the default button. The React rebuild should reproduce Enter-to-advance — Iranian
data-entry operators expect it throughout this application.

**Gaps:** no validation that the six accounts are set (only `CA_DMaliat` non-empty); no VAT-rate
range check; no delete; no `is_active`; no permission check.

---

### 13.2 Item list — `AnbarCalaU`

Fully specified at §2.5. Summary: warehouse selector built at runtime as a popup menu, grid over
`Anbar_AjnasView(@ID)`, five working buttons plus the **dead `A_Resome`** (`سابقه`, "history").
No permission checks anywhere on this screen.

### 13.3 Item editor — `AnbarCalaAddU`

Fully specified at §2.2–§2.3. Controls and Persian labels at §1.2; validations at §2.2; defaults
at §2.2.2. Two `TsSlider` toggles (`AJ_Maliat`, `AJ_Manfi`) captioned `بله`/`خیر`. Window geometry
is persisted to the INI file (`:204-218`), as on every screen in this module.

### 13.4 Item picker — `AnbarCalaSelectU`

Fully specified at §2.6. One search box (live, per-keystroke, `PATINDEX`-based, term truncated at
18 characters), a grid, `BitBtn1` (OK, enabled only when there are results), `BitBtn2` (cancel),
double-click = OK. Returns via `Tag = 1` and the caller reads
`Dm.AnbarCala_SeekName.FieldByName('AJ_Code')`.

---

### 13.5 Invoice list — `AnbarListU`

**Purpose:** the module's hub. Lists `Anbar_Factor` for the current fiscal year with computed
payment and e-invoicing columns, and launches every invoice operation.

**Query** (`Reload`, `AnbarListU.pas:520-549`), rebuilt on every filter change:

```sql
Select [Top <N>] *
 , Send= (Select Count(*) from Moadian Where moadian.M_link=Anbar_Factor.AF_ssn and Moadian.M_id=1)
 , payF= (Select Sum(S_Mab) From DFish  Where S_Linkprg=1 and S_Coid=AF_Coid and S_LinkSSN=AF_Factor )
 , payC= (Select Sum(S_Mab) From DCheck Where S_Linkprg=1 and S_Coid=AF_Coid and S_LinkSSN=AF_Factor )
 , Af_typeN = ( case when AF_Type = 1 then 'رسید انبار'  when AF_Type = 2 then 'حواله انبار'
                     when AF_Type = 3 then 'برگشت از خرید'  when AF_Type = 4 then 'برگشت از فروش'
                else '' end )
 From Anbar_Factor
   where AF_COID=<year>
   [and AF_Type= <type>]
 Order By AF_Date Desc, AF_Factor Desc
```

**Filters:**

| Control | Purpose | Values |
|---|---|---|
| `B_Type` → `POP1` (`Type0`…`Type4`) | document type | `_Type` 0 = all, 1–4 = §3.1. `Type0Click` reads `(Sender as TMenuItem).Tag` (`:258-262`) |
| `S_Search` → `POP2` (`p2_All`, `P2_10`, `P2_25`, `P2_50`) | row limit | `_All` 0 = all, else `Top N`. Default 25 (`:146`) |
| `GridFontSize: TsUpDown` | grid font size 6–15 | `:199-205`, persisted to INI |

> The `S_Search` button is captioned as a *search* affordance but opens the **row-limit** menu
> (`:212-216`). There is no text search on this screen at all. Misleading, not dead.

**Grid `G1` columns** (`AnbarListU.dfm`), over the query above: `AF_Factor`, `AF_Date`,
`Af_typeN`, `AF_CustomerN`, `AF_Sanad`, `AF_Mab`, `AF_Kasr`, `AF_Maliat`, `AF_Total`, `payF`,
`payC`, `Send`, `AF_Desc`. Column widths persisted per index to the INI file (`:496-497, 510-511`).

`Q1AF_TypeGetText` (`:174-186`) provides a second, client-side copy of the type labels — a third
place the same mapping is written (§5.1.1).

**Buttons** (all wired; permissions applied by enabling/disabling in `init`, `:151-158`):

| Button | Persian caption | English | Handler | Permission |
|---|---|---|---|---|
| `Anbar2` | `فاکتور جدید` | New invoice | opens `PopupMenu1` → `T1`/`T2`/`T3`/`T4` | per type: 1404/1405/1407/1406 |
| `Anbar0` | `اصلاح فاکتور` | Edit invoice | `Anbar0Click` (`:269-283`) → `EditFactor` | 1408 |
| `AR_Chap` | `چاپ فاکتور` | Print invoice | → `Factorprint3.init` | 1410 |
| `AR_Chap2` | `چاپ فاکتور رسمی` | Print official invoice | → `Factorprint2.init` | none |
| `sButton8` | `چاپ لیست` | Print list | `AR_Chap3Click` (`:292-300`) → FastReport `RP1` | none |
| `AR_Charp4` | `مشاهده فاکتور` | View invoice | `AR_Chap4Click` → `ViewFactor` | none |
| `AR_Delete` | `حذف فاکتور` | Delete invoice | `AR_DeleteClick` (§4.2.5) | 1414 |
| `AR_ReNo` | `تغییر شماره فاکتور` | Change invoice number | `AR_ReNoClick` (§4.2.6) | none |
| `AR_Variz` | `تسویه فاکتور` | Settle invoice | `AR_VarizClick` (§9) | 2115 or 2101 |
| `sButton4` | `برگشت` | Back | `sButton7Click` → `Close` |  |

**Guards:** `Q1BeforeDelete` → `abort` (`:188-191`) and `G1KeyDown` swallowing Ctrl+Delete
(`:193-197`) prevent grid-level deletion. `Is_New_Sanad_Valid` fronts every mutating action.

> **Note the permission gaps**: renumber, official-invoice print, view and list-print have **no
> permission code at all**. Renumbering is a data-modifying operation (§4.2.6) available to anyone
> who can open the screen.

---

### 13.6 Invoice editor — `AnbarFactorU`

Form caption `فاکتور کالا` ("goods invoice"). Lifecycle at §4; save path at §4.2.2.

**Header panel:**

| Control | Persian label | Type | Notes |
|---|---|---|---|
| `L_REG` | — | label | `DM.RegName` — the operating entity's name |
| `AF_Type` | — | label | the type name from `TypeC[]` (§5.1.1) |
| `AF_Date` | `تاریخ فاکتور` | `TFullDate` | Jalali; the binary control (§14) |
| `AF_Factor` | `شماره فاکتور` | `TEditInt` | **read-only** (`ClearForm`, `:685`) — allocated on save |
| `AF_Sanad` | `شماره سند` | `TEditInt` | **read-only** (`:686`) |
| `S_Bed` + `B_Bed` + `N_Bed` | `: طرف حساب` | edit + `...` button + memo | counterparty account. `S_BedChange` (`:356-362`) resolves via `Taraf` on every keystroke; `B_BedClick` (`:541-553`) opens the picker, **temporarily nils `OnChange`** to avoid re-entrancy |
| `AF_Desc` | `توضیحات فاکتور` | `TsMemo` | narration; `ReadOnly` when `State > 2` or the voucher is frozen (`UpdateState`, `:483`) |

**Line grid `G1`** (`AnbarFactorU.dfm:345-422`) over the in-memory `CDS1: TVirtualTable`:

| Column | Persian title | English |
|---|---|---|
| `Code` | `کد کالا` | Item code |
| `Name` | `نام کالا` | Item name |
| `prop` | `مشخصه` | Specification |
| `vahed` | `واحد` | Unit |
| `Num` | `تعداد` | Quantity |
| `Phi` | `فی` | Unit price |
| `Kol` | `مبلغ` | Gross amount |
| `kasr` | `تخفیف` | Discount |
| `Maliat` | `مالیات` | VAT |
| `Total` | `مبلغ کل` | Line total |

`G1.RecalculateSummaryResults(true)` is called after every mutation — the footer totals are
client-side.

**Buttons:** `B_Add` (`اضافه`), `B_Edit` (`اصلاح`), `B_Delete` (`حذف`), `B_Save` (`ذخیره`),
`B_Exit` (`برگشت`). Add/Edit/Delete guard on `State > 2` and show
`اجازه تغييرات در فاکتور را نداريد` ("you are not permitted to change the invoice") — but see the
sticky-`State` defect at §4.2.4.

**Popup menus:**

| Menu | Item | Caption | Action |
|---|---|---|---|
| `PopupMenu1` | `OP1` | `لیست مانده با متوسط قیمت تمام شده` | Load stock on hand as invoice lines — §6.4 |
| | `OP2` | `ذخیره اقلام وارد شده در فایل` | Export lines to an INI file — §4.3 |
| | `OP3` | `بارگزاری اقلام ذخیره شده قبلی` | Import lines from an INI file — §4.3 |
| `P_Bed` | `N1` | `ذخیره به عنوان پیش فرض` | Save the current counterparty code as the default (`:131-134`) |

**Dead / disabled on this screen:**

- **Four commented-out settlement handlers** `D_NaghdiClick`, `D_EditClick`, `D_checkClick`,
  `D_DeleteClick` (`:76-79` declarations, `:364-451` bodies) — the abandoned inline-settlement
  panel (§9.6). Their `T_panel` is also commented out (`:152`).
- **The default-customer block** in `NewFactor` (`:154-174`) — commented out.
- **`Q1: TADOQuery`** with a third stock-balance implementation — dead (§5.1.4).
- **`PrintFactor`** (`:343-354`) — no caller.
- **`CDS2: TABSTable` / `DS2` / `DS3`** — declared (`:38-42`), never used.
- **`G1AfterScanDataset`** (`:125-129`) — body is a single commented line.

---


---

[← 12. SQL and stored procedures (part b)](05-12-b-sql-and-stored-procedures.md) | [index](00-index.md) | [13. Screen specifications (part b) →](05-13-b-screen-specifications.md)
