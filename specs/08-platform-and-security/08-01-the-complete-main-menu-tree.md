_Part of [08-platform-and-security](../08-platform-and-security.md) — [index](00-index.md)_

## 1. The complete main-menu tree

### 1.1 Shape of the navigation

There is **no `TMainMenu`** in this application. Navigation is a **ribbon**: a
`TsPageControl` named `PC1` docked to the top of the main form (`Mainu.dfm:303-323`,
`Align = alTop`, `Height = 97`, `TabHeight = 24`), containing six `TsTabSheet` pages.
Each page holds a horizontal strip of `TsSpeedButton` tool buttons separated by 7px
"griph" separator panels (`sPanel4`, `sPanel5`, … — these are decoration only, no behaviour).

Four buttons carry a `DropdownMenu` (a `TPopupMenu`), which is where the only true
*menus* in the product live: `POP_Sarfasl`, `POP_Moein`, `Pop_Report`, `POP_AnbarFactor`,
`POP_AnbarReport`.

A bottom status panel `PX` (`Mainu.dfm:129-136`) holds the company/fiscal-year label
`B_Company` and a duplicate Exit button `B_Exit`.

**Shortcut keys: there are none.** No `ShortCut` property appears anywhere in
`Mainu.dfm` — verified by scanning the whole file. Every navigation action is
mouse-only. The React rebuild should introduce keyboard access (see §14).

**Enablement:** the ribbon is *not* statically enabled. `TMain.Reload`
(`Mainu.pas:884-968`) sets `.Enabled` on ~35 controls from the permission table on
every login and after every visit to the admin screen (`Mainu.pas:970-974`).
Everything not listed in `Reload` is permanently enabled for every logged-in user.

Below, `→` means "opens", and the *Perm* column is the numeric permission id checked in
`TMain.Reload`; `—` means **no permission check at all**.

---

### 1.2 Ribbon tab 1 — `TS1` · `اسناد حسابداري` · "Accounting Documents"

`Mainu.dfm:324-1138`

| Control | Persian caption | English | Opens | Perm | Enforced at |
|---|---|---|---|---|---|
| `Sarfasl_H` | `سرفصلهاي حسابداري` | Chart of accounts | *(dropdown → `POP_Sarfasl`)* | 1100 | `Mainu.pas:907` |
| `Asnad_Moein` | `اسناد معین` | Subsidiary-ledger documents | *(dropdown → `POP_Moein`)* | 1112 | `Mainu.pas:912` |
| `Asnad_rooznameh` | `اسناد روزنامه` | Journal documents | `RooznamehView.init` | 1132 | `Mainu.pas:925`, handler `Mainu.pas:1023-1026` |
| `Report` | `گزارش` | Reports | *(dropdown → `Pop_Report`)* | 1122 | `Mainu.pas:927` |
| `B_CardJari` | `خلاصه\nکارت` | Current-account card summary | `CardJariF.init` | 1131 | `Mainu.pas:920`, handler `Mainu.pas:443-446` |
| `B_Report7` | `رویت\nجامع` | Comprehensive view | `RoyatJF.init` | — | `Mainu.pas:438-441` |
| `B_Report9` | `دفتر\nتجمیعی` | Aggregated ledger | `DaftarT_F.init` | — | `Mainu.pas:604-607` |
| `B_Report3` | `جستجو\nدفاتر` | Search ledgers | `Moeinsearch.init` | — | `Mainu.pas:433-436` |
| `B_Enteghal1` | `بستن حسابها` | Close accounts (year-end) | `NewFinalf.init` | Admin-only | `Mainu.pas:958`, handler `Mainu.pas:624-627` |
| `B_Enteghal2` | `انتقال موجودی انبار` | Transfer inventory balance | *(none — `Visible=False`)* | Admin-only | `Mainu.dfm:808`, `Mainu.pas:959` |
| `B_Enteghal3` | `انتقال مانده` | Carry forward balances | `EnteghalF.init` | Admin-only | `Mainu.pas:960`, handler `Mainu.pas:629-632` |

#### Dropdown `POP_Sarfasl` (from `Sarfasl_H`) — `Mainu.dfm:10637-10665`

| Item | Persian | English | Opens | Perm | Enforced |
|---|---|---|---|---|---|
| `Sarfasl_List` (Tag 1101) | `ليست سرفصلها` | Chart-of-accounts list | `Snew.init` then `DM.UpdateTable` | 1101 | `Mainu.pas:908`, handler `Mainu.pas:566-571` |
| `Sarfasl_Add` (Tag 1102) | `ايجاد سرفصل` | Create account | **dead** — handler starts with `exit` | hard-wired `False` | `Mainu.pas:909` (`:= False`), handler `Mainu.pas:573-578` |
| `Jari_List` | `جاري اشخاص` | Personal current accounts | `Sahamdar.init` | 1105 | `Mainu.pas:910`, handler `Mainu.pas:986-989` |

#### Dropdown `POP_Moein` (from `Asnad_Moein`, also `PopupMenu` of `Sarfasl_H`) — `Mainu.dfm:10557-10636`

| Item | Persian | English | Opens | Perm | Enforced |
|---|---|---|---|---|---|
| `SMoein1` (Tag 1212) | `اسناد معين درحال تحرير` | Docs in drafting | `SanadView.ViewAsnad(0)` | **1127** | `Mainu.pas:913`, handler `Mainu.pas:458-461` |
| `SMoein2` (Tag 1213) | `اسناد معين تاييد شده` | Approved docs | `SanadView.ViewAsnad(1)` | **1125** | `Mainu.pas:914`, handler `Mainu.pas:463-466` |
| `SMoein3` (Tag 1214) | `اسناد معين ثبت شده` | Permanently posted docs | `SanadView.ViewAsnad(2)` | **1126** | `Mainu.pas:915`, handler `Mainu.pas:468-471` |
| `SMoein0` (Tag 1201) | `سند معين جديد` | New subsidiary document | guard `DM.Is_New_Sanad_Valid(CO_ID)` then `SanadEditF.new` | 1113 | `Mainu.pas:916`, handler `Mainu.pas:676-682` |
| `SMoein4` (Tag 1209) | `مشاهده سند معین` | View document | `SanadEditF.View(0)` | 1121 | `Mainu.pas:917`, handler `Mainu.pas:580-584` |
| `SMoein5` | `انتقال اسناد به Excell` | Export documents to Excel | **dead** — body fully commented out | — | `Mainu.pas:991-995` |
| `SMoein6` | `خلاصه اسناد معین` | Document summary | `MoeinZip.init` | 1128 | `Mainu.pas:918`, handler `Mainu.pas:996-999` |
| `EBC` | `ایجاد دفاتر الکترونیکی دارایی` | Generate tax-authority e-ledgers | `ToExcelDaraei.init` | — | `Mainu.pas:416-419` |
| `SMoein7` | `کپی سند معین` | Copy document | `GetNo('کپی سند','شماره سند')` → `SanadEditF.Copy(N)`, guarded by `Is_New_Sanad_Valid` | — | `Mainu.pas:1001-1011` |

> **Note the crossed wiring:** the *drafting* item is gated by 1127 while the admin
> checkbox labelled "ليست اسناد درحال تحرير" *is* `C1127`, and "ليست اسناد تاييد شده"
> is `C1125`, "ليست اسناد ثبت شده" is `C1126`. So `SMoein1/2/3 → 1127/1125/1126`
> is **correct**, but the ids are non-monotonic relative to the visual order.
> The menu-item `Tag` values (1212/1213/1214/1201/1209) are **not** used for
> permission lookup — they are vestigial. Do not carry the Tags forward.

#### Dropdown `Pop_Report` (from `Report`) — `Mainu.dfm:10666-10730`

| Item | Persian | English | Opens | Perm | Enforced |
|---|---|---|---|---|---|
| `Report1` (Tag 1215) | `مشاهده دفتر معين` | View subsidiary ledger | `DMoeinF.init(0,0,0,0)` | 1123 | `Mainu.pas:921`, handler `Mainu.pas:634-637` |
| `_Report9` | `مشاهده دفتر تجمیعی` | View aggregated ledger | `TajmiF.initM` — **`Visible = False`** | — | `Mainu.dfm:10683`, handler `Mainu.pas:671-674` |
| `Report2` (Tag 1216) | `تراز آزمايشي 4 ستوني` | 4-column trial balance | `Taraz4Setooni.init` | 1124 | `Mainu.pas:922`, handler `Mainu.pas:639-643` |
| `Report5` | `تراز آزمایشی 6 ستونی` | 6-column trial balance | `Taraz6Setooni.init` | — | `Mainu.pas:650-653` |
| `Report4` | `لیست کنترلی` | Control list | `KolState.init` | — | `Mainu.pas:645-648` |
| `Report6` | `کنترل شماره اسناد` | Document-number control | `Report6F.init` | — | `Mainu.pas:655-658` |
| `Report8` | `بدهکاران و بستانکاران` | Debtors & creditors | `BedBesF.init` | — | `Mainu.pas:660-663` |
| `Report9` | `دفتر کل` | General ledger | `DKolF.init` | 1141 | `Mainu.pas:923`, handler `Mainu.pas:665-669` |

---

### 1.3 Ribbon tab 2 — `sTabSheet1` · `خزانه` · "Treasury"

`Mainu.dfm:1139-1296`

| Control | Persian | English | Opens | Perm | Enforced |
|---|---|---|---|---|---|
| `B_SodoorCheck` | `صدور چک` | Issue cheque | `CheckListF.init` | 2110 | `Mainu.pas:951`, handler `Mainu.pas:609-612` |
| `B_DaryaftCheck` | `دریافت چک` | Receive cheque | `CheckListDF.init` | 2101 | `Mainu.pas:950`, handler `Mainu.pas:522-525` |
| `B_Variz` | `واریز نقدی` | Cash deposit | `FishListDF.init` | 2115 | `Mainu.pas:952`, handler `Mainu.pas:619-622` |
| `B_Tankhah` | `لیست تنخواه` | Petty-cash list | `tankhahlistF.init` | 2120 | `Mainu.pas:953`, handler `Mainu.pas:614-617` |
| `B_Bardasht` | `برداشت نقدی` | Cash withdrawal | *(none)* — `Enabled=False`, `Visible=False` | — | `Mainu.dfm:1178-1180` |
| `Bank_Tanzim` | `گزارش` | Report | *(none)* — `Visible=False` | — | `Mainu.dfm:1150` |

---

### 1.4 Ribbon tab 3 — `sTabSheet6` · `انبارداري` · "Warehousing"

`Mainu.dfm:1297-1587`. This is the ribbon's `ActivePage` as designed (`Mainu.dfm:308`),
but `FormCreate` overrides it to `TS1` at runtime (`Mainu.pas:350-355`).

| Control | Persian | English | Opens | Perm | Enforced |
|---|---|---|---|---|---|
| `Anbar_Tanzim` (Tag 1401) | `تنطيمات انبار` | Warehouse settings | `AnbarTanzim.init` | 1401 | `Mainu.pas:929`, handler `Mainu.pas:534-537` |
| `Anbar_Ajnas` (Tag 1402) | `معرفي اجناس` | Item master | `AnbarCala.init(0)` | 1402 | `Mainu.pas:930`, handler `Mainu.pas:539-542` |
| `Anbar_Factor` | `صدور فاکتور` | Issue invoice | *(dropdown → `POP_AnbarFactor`)* | 1403 | `Mainu.pas:931` |
| `Anbar_FactorList` | `لیست فاکتورها` | Invoice list | `AnbarList_F.init` | — | `Mainu.pas:554-557` |
| `Anbar_Report` | `گزارش ورود و خروج` | Goods in/out report | `Anbar_AmalkardF.init` (also has dropdown `POP_AnbarReport`) | 1409 | `Mainu.pas:938`, handler `Mainu.pas:559-564` |
| `AR_Jensi` | `کارت جنسی` | Stock card | `AnbarCardJensi.init` | 1412 | `Mainu.pas:942`, handler `Mainu.pas:549-552` |
| `AR_Amalkard` | `عملکرد انبار` | Warehouse performance | `Anbar_MandehF.init` | 1413 | `Mainu.pas:943`, handler `Mainu.pas:544-547` |
| `AR_Print1` | `چاپ فاکتور` | Print invoice | `FactorPrint3.init(0)` | 1410 | `Mainu.pas:939`, handler `Mainu.pas:1013-1016` |
| `AR_Print2` | `فاکتور رسمی` | Official (VAT) invoice | `FactorPrint2.init(0)` | 1410 | `Mainu.pas:940`, handler `Mainu.pas:1018-1021` |

#### Dropdown `POP_AnbarFactor` — `Mainu.dfm:10731-10774`

All four "issue" items share the handler `Anbar2Click` (`Mainu.pas:689-695`), which
reads `(Sender as TMenuItem).Tag` as the *document kind* and calls
`AnbarFactor.NewFactor(tag)` — after the `Is_New_Sanad_Valid` guard.

| Item | Tag (= doc kind) | Persian | English | Perm | Enforced |
|---|---|---|---|---|---|
| `Anbar2` | 2 | `صدور حواله انبار` | Issue goods-issue note | 1405 | `Mainu.pas:933` |
| `Anbar1` | 1 | `صدور رسید انبار` | Issue goods-receipt note | 1404 | `Mainu.pas:932` |
| `Anbar4` | 4 | `برگشت حواله انبار` | Return of goods-issue | 1406 | `Mainu.pas:934` |
| `Anbar3` | 3 | `برگشت رسید انبار` | Return of goods-receipt | 1407 | `Mainu.pas:935` |
| `Anbar0` | — | `اصلاح` | Amend | 1408 | `Mainu.pas:936`, handler `Mainu.pas:697-700` → `AnbarFactor.EditFactor(0)` |

> ⚠️ The visible captions and the permission ids are **swapped relative to the labels
> in the admin screen**: `C1404` is captioned `صدور فاکتور خريد` ("issue purchase
> invoice") but gates `Anbar1` = `صدور رسید انبار` ("goods receipt"); `C1405` is
> `صدور فاکتور فروش` ("issue sales invoice") but gates `Anbar2` = `صدور حواله انبار`
> ("goods issue"). Receipt≈purchase and issue≈sale, so it is *semantically*
> consistent, but the label text will confuse anyone porting it. Fix the naming in
> the rebuild; keep the mapping.

#### Dropdown `POP_AnbarReport` — `Mainu.dfm:10775-10790`

| Item | Persian | English | Opens | Perm |
|---|---|---|---|---|
| `AR_Chap2` | `فاکتور رسمی` | Official invoice | **no `OnClick` — dead item** | — |
| `AR_Kholaseh` | `گزارش ورود و خرج` | Goods in/out summary | **no `OnClick` — dead item** | 1411 (`Mainu.pas:941`) |

---

### 1.5 Ribbon tab 4 — `TS_Pesteh` · `عملیات خرید پسته` · "Pistachio Purchase Operations"

`Mainu.dfm:1588-1668`.
**Conditionally hidden:** in `FormActivate`, if `Length(DM.Anbar_DB) = 0` the tab's
caption is blanked and the tab is disabled (`Mainu.pas:325-329`). `DM.Anbar_DB` is
resolved at startup by asking SQL Server whether a database named `Anbar` exists
(`Dmu.pas:765-777`). So this whole feature area is **licensed by database presence**,
not by user permission.

| Control | Persian | English | Opens | Perm |
|---|---|---|---|---|
| `B_Anbar` | `عملیات انبار` | Warehouse operations | `SodoorSanad.init` (`Mainu.pas:976-979`) | — |
| `B_Kharid` | `خرید پسته` | Pistachio purchase | `FactorPesteh_F.init` (`Mainu.pas:511-514`) | — |
| `B_AnbarReport` | `گزارش عملکرد` | Performance report | `AnbarReport_F.init` (`Mainu.pas:981-984`) | — |

---

### 1.6 Ribbon tab 5 — `sTabSheet2` · `امکانات برنامه` · "Application Facilities"

`Mainu.dfm:1669-2002`

| Control | Persian | English | Opens | Perm | Enforced |
|---|---|---|---|---|---|
| `B_Backup` | `ايجاد\n پشتيبان` | Create backup | `BackupForm.init` | 1110 | `Mainu.pas:947`, handler `Mainu.pas:372-375` |
| `B_Form` | `تنظیمات فرمها` | Form/print settings | `TanzimChap.init` | — | `Mainu.pas:506-509` |
| `B_Setting` | `تنطیمات سیستم` | System settings | `Tanzimf.init` | 1109 | `Mainu.pas:946`, handler `Mainu.pas:377-380` |
| `B_Skin` | `تغيير \nپوسته` | Change skin/theme | `SelectSkin(Sk1)` + persist to ini | 1111 | `Mainu.pas:948`, handler `Mainu.pas:382-391` |
| `B_Pass` | `تغيير \nسال مالي` | **Change fiscal year** (caption is misleading — the button is named `B_Pass` but does *not* change a password) | `Changes_F.init` then reload company header | — | `Mainu.pas:421-431` |
| `sSpeedButton1` | `تغییر رمز` | Change password | `changepassword.init` | — | `Mainu.pas:599-602` |
| `B_Close` | `خروج` | Exit | `Close` (with confirm dialog) | — | `Mainu.pas:448-451` |
| `B_Admin` | *(icon only)* | User & permission admin | `AdminF.init` then `Reload` | **`Supervisor = 1`** | `Mainu.pas:957`, handler `Mainu.pas:970-974` |
| `B_Add` | *(icon only)* | New company / fiscal year | `MakeNew.init` | hard-wired `False` | `Mainu.pas:961`, handler `Mainu.pas:496-499` |
| `B_CloseMoein` | *(icon only)* | Close accounts (`BastanhesabF`) | `BastanhesabF.init` — **`Visible = False`** | commented out | `Mainu.dfm:1756`, `Mainu.pas:963`, handler `Mainu.pas:453-456` |

> **Hidden developer back door.** `B_Add` (create new fiscal year/company) is disabled
> at design time *and* forced `False` in `Reload` (`Mainu.pas:961`). It is re-enabled by
> a **drag-and-drop gesture**: hold `Ctrl+Alt` and mouse-down on `B_Exit`
> (`Mainu.pas:501-504`) to start a drag, then drop it on the `B_Company` label
> (`Mainu.pas:527-532`), which toggles `B_Add.Enabled`. `B_Company.OnDragOver` accepts
> any `TsSpeedButton` (`Mainu.pas:516-520`).
> ⛔ **DO NOT PORT.** This is an undocumented privilege-escalation gesture that bypasses
> the permission system entirely.

---

### 1.7 Ribbon tab 6 — `sTabSheet3` · `Information`

`Mainu.dfm:2003-2244`. Static "about" panel: Product Name / Version / Last Update /
E-Mail plus a logo image. No actions.

### 1.8 Non-ribbon / debug controls on the main form

All are `Visible = False` and should **not** be reproduced:

| Control | Caption | Handler | Status |
|---|---|---|---|
| `Button1` | `Sarfasl` | `Mainu.pas:702-757` | One-shot 2012 data-migration script, entirely commented out |
| `Button2` | `Moein 92` | `Mainu.pas:759-801` | One-shot migration of the 1391 subsidiary ledger, commented out |
| `Button5` | `Sahamdar` | `Mainu.pas:803-844` | One-shot shareholder→chart-of-accounts migration, commented out |
| `Button3` | `اصلاح مشخصات` | `Mainu.pas:357-360` → `SahamdarP_F.Edit(0)` | Orphan |
| `Button4` | `اطلاعات پايه` | `Mainu.pas:367-370` → `Kharid_B.init` | Orphan (inside hidden `Panel3`) |
| `Button8` | `خريد پسته` | `Mainu.pas:362-365` → `Kharid.New_Kharid` | Orphan (inside hidden `Panel3`) |
| `Button6` | `اشخاص` | `Mainu.pas:846-849` → `Sarfasl_kol.init` | Orphan |
| `Button7` | `Demo = 306239` | `Mainu.pas:851-882` | Developer scratch pad: runs a 20-step fake progress bar then `exit`; the rest is dead code including `SanadEditF.Import` and an `ElfHash('Demo')` probe |
| `Memo1` | — | `Mainu.dfm:4313-4383` | **Valuable documentation**: the `Moein.M_ID` → source-table mapping (M_ID 1=warehouse invoice, 21=cheque received, 22=cheque to bank, 23=cheque cleared, 24=cheque returned to owner, 25=cash-slip deposit, 26=cheque issued, 27=cheque bounced, 31–36=purchase/sale/return kinds, 41=petty cash). Transcribe this into the domain model. |
| `Memo2` | — | `Mainu.dfm:4394-4411` | List of the nine asset/liability report categories |
| `ED1` | — | `Mainu.dfm:4298-4312` | Hidden `TFullDate` used purely as a Gregorian→Jalali converter by `DoBackup` (`Mainu.pas:398-401`) |
| `SpeedButton1/2/3`, `Shape1` | — | `Mainu.dfm:26-91` | Design-time leftovers, no handlers |

---


---

Prev: [index](00-index.md) · Next: [2. Application startup sequence](08-02-application-startup-sequence.md)
