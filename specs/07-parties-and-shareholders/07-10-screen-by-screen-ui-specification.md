_Part of [07-parties-and-shareholders](../07-parties-and-shareholders.md) — [index](00-index.md)_

## 10. Screen-by-screen UI specification for the React rebuild

Common legacy behaviours to **drop** in the rebuild: per-form INI persistence of window
geometry and grid column widths (`SahamdarU.pas:274-299`, `SahamdarEditU.pas:190-211`,
`TarafU.pas:87-102`, `SahamdarInfoU.pas:53-73`, `CardJariU.pas:203-212`). Replace with normal
responsive layout and, optionally, per-user table preferences.

Common legacy behaviours to **keep**: RTL layout (`BiDiMode = bdRightToLeft` on every form), Jalali
dates as text, digit-only inputs, `Enter`/`↓` = next field, `↑` = previous field.

### 10.1 `PartyRegister` — persons & legal entities (`SahamdarU`)

* **Legacy:** `TSahamdar`, caption `اطلاعات تکمیلی اشخاص` = "Supplementary information of persons"
  (`SahamdarU.dfm:5`).
* **Entry:** main menu `جاری اشخاص` ("Person current accounts") — `Mainu.dfm:10661-10664`,
  `Mainu.pas:986-989` → `Sahamdar.init`. Menu item requires permission **1105**
  (`Mainu.pas:910`), Persian caption `جاری اشخاص`.
* **Purpose:** browse / create / edit the party master, in two tabs.

**Tab switcher** — two large toggle buttons (`SahamdarU.dfm:123-154`):

| Button | Caption | English | Sets |
|---|---|---|---|
| `Select_1` | `اشخاص` | Persons | `_HaHo := 1` (`SahamdarU.pas:203-207`) |
| `Select_2` | `شرکتها` | Companies | `_HaHo := 2` (`SahamdarU.pas:209-213`) |

Only one grid is visible at a time; both bind the same datasource (`SahamdarU.pas:178-196`).

**Grid — Persons (`G1`, `SahamdarU.dfm:276-332`):**

| # | Field | Persian header | English header |
|---|---|---|---|
| 1 | `S_Card` | `عضویت` | Membership / card no. |
| 2 | `S_Name` | `نام` | First name |
| 3 | `S_Famil` | `نام خانوادگی` | Surname |
| 4 | `S_Father` | `نام پدر` | Father's name |
| 5 | `S_CodeMelli` | `کد ملی` | National ID |
| 6 | `S_Mobile` | `شماره موبایل` | Mobile number |
| 7 | `S_Lock` | `?` | Lock (padlock icon, §below) |

**Grid — Companies (`G2`, `SahamdarU.dfm:65-113`):**

| # | Field | Persian header | English header |
|---|---|---|---|
| 1 | `S_Card` | `عضویت` | Membership / card no. |
| 2 | `S_Name` | `نام شرکت` | Company name |
| 3 | `S_Famil` | `نام مدیر یا نماینده` | Manager / representative name |
| 4 | `S_CodeMelli` | `شناسه ملی` | Legal-entity national ID |
| 5 | `S_Mobile` | `شماره موبایل` | Mobile number |
| 6 | `S_Lock` | `?` | Lock |

The `S_Lock` cell is custom-drawn as an open/closed padlock bitmap
(`SahamdarU.pas:301-333` for `G1`, `:335-368` for `G2`; `Image0` = unlocked, `Image1` = locked,
bitmap embedded at `SahamdarU.dfm:166-201`).

**Buttons (right rail, `SahamdarU.dfm:335-438`):**

| Control | Persian | English | Permission | Handler |
|---|---|---|---|---|
| `B_New` | `جدید` | New | 1106 (`افزودن جاری جدید` = "Add new current account", `Admin.dfm:115-125`) | `SahamdarU.pas:215-234` |
| `B_Select` | `تایید` | Confirm/select | — (shown only in picker mode) | `SahamdarU.pas:265-272` |
| `B_Edit` | `اصلاح` | Edit | 1107 (`اصلاح جاری` = "Edit current account", `Admin.dfm:127-137`) | `SahamdarU.pas:243-262` |
| `B_Delete` | `حذف` | Delete | 1108 (`حذف جاری` = "Delete current account", `Admin.dfm:139-151`, itself `Visible = False`) | **none — dead button** |
| `B_Bank` | `حساب بانکی` | Bank account | 1107 (note: DFM `Tag` says 1108, code uses 1107 — `SahamdarU.pas:110`) | `SahamdarU.pas:236-241` |
| `sBitBtn4` | `برگشت` | Back | — | `SahamdarU.pas:73-76` |
| `B_Lock` (toolbar) | hint `چاپ` ("print" — wrong hint) | Toggle lock | Admin only (`SahamdarU.pas:112-113`) | `SahamdarU.pas:78-93` |

**Context menu**, only for `Dm.userId = 68` (`SahamdarU.pas:101-105`, `SahamdarU.dfm:470-485`):
`انتقال به جاری شرکتها` / `انتقال به جاری اشخاص`.

**Two modes:**

* `init` (`SahamdarU.pas:95-117`) — management mode; New/Edit/Delete/Bank per permission; Select
  hidden.
* `init2` (`SahamdarU.pas:119-134`) — picker mode; all CRUD hidden, Select shown; the chosen
  `S_Card` is returned via the form's `Tag` (`SahamdarU.pas:267-271`), `0` = cancelled.

**New/Edit routing** (`SahamdarU.pas:215-262`): tab 1 → `SahamdarEdit`, tab 2 → `CompanyEdit`;
afterwards the list is reopened and repositioned on the affected card.

**React notes:** one route `/parties` with a `kind` tab (`person` | `company`); a `select` mode for
modal use; server-side pagination and search (the legacy grid loads the entire table and filters
client-side via the `rDBGrid_MS` filter bar, `SahamdarU.dfm:53`).

### 10.2 `PersonEditor` (`SahamdarEditU`)

* **Legacy:** `TSahamdarEdit`, caption `ورود اطلاعات اشخاص` = "Person data entry"
  (`SahamdarEditU.dfm:5`).
* **Purpose:** create/update a natural person and choose which control accounts it gets a detail
  account under.

**Fields (right column, tab order as in the DFM):**

| Tab | Control | Persian label | English | Type / constraint |
|---|---|---|---|---|
| 0 | `SCard` | `ش.شناسايي` | Identification (card) number | integer, max 8 digits (`SahamdarEditU.dfm:292`); **read-only when editing** (`SahamdarEditU.pas:164`) |
| 1 | `SName` | `نام` | First name | text, required |
| 2 | `SFamil` | `نام خانوادگی` | Surname | text, required |
| 3 | `SFather` | `نام پدر` | Father's name | text, required |
| 4 | `SIDNO` | `ش.شناسنامه` | ID document number | integer, max 11 digits (`SahamdarEditU.dfm:345`) |
| 5 | `SMobile` | `موبایل` | Mobile | text |
| 6 | `SBDate` | `تاریخ تولد` | Birth date | Jalali text, keystrokes `[/0-9⌫]` (`SahamdarEditU.pas:364-370`) |
| 7 | `SBPlace` | `محل تولد` | Birth place | text |
| 8 | `SSDate` | `تاریخ صدور` | ID issue date | Jalali text, same filter |
| 9 | `SSPlace` | `محل صدور` | ID issue place | text |
| 10 | `SCodeMelli` | `کد ملی` | National ID | text, unique |
| 11 | `SCodePosti` | `کدپستی` | Postal code | text |
| 12 | `SAddress` | `آدرس` | Address | text, wide |
| 16 | `SCodeSabt` | `کد ثبت` | Registration code | text, max 12 (`SahamdarEditU.dfm:424`) |
| 17 | `SMaliatState` | `وضعیت مالیاتی` | Tax status | dropdown, 5 options (§4.2) |

**Grid `G1` (left, `SahamdarEditU.dfm:359-417`)** — the control-account chooser:

| Field | Persian header | English | Editable |
|---|---|---|---|
| `M_L` | `کد گروه` | Group code (`Kol-Moein`) | no |
| `S_Name` | `نام گروه` | Group name | no |
| `S_Found` | `عضو` | Member (checkbox) | **yes** |

**Buttons:** `B_Save` = `ذخیره` ("Save", `SahamdarEditU.dfm:200`), `B_Exit` = `برگشت` ("Back",
`:191`).

**Behaviour:** typing in `SCard` re-runs `Open_Coding` on every keystroke
(`SahamdarEditU.pas:343-346`) — debounce this in React. `↑`/`↓`/`Enter` navigate between fields
(`SahamdarEditU.pas:348-362`). Save validations in §3.2; on success, ticked rows are materialised as
accounts (§2.4) and the form closes.

### 10.3 `LegalEntityEditor` (`CompanyEditU`)

* **Legacy:** `TCompanyEdit`, caption `ورود اطلاعات اشخاص حقوقی` = "Legal-entity data entry"
  (`CompanyEditU.dfm:5`).
* Same layout minus the person-specific fields.

| Tab | Control | Persian label | English |
|---|---|---|---|
| 0 | `SCard` | `ش شناسايي` | Identification (card) number |
| 1 | `SName` | `نام شخصیت` | Entity name |
| 2 | `SFamil` | `نام مدیر یا نماینده` | Manager / representative |
| 3 | `SMobile` | `شماره تماس` | Contact number |
| 4 | `SBDate` | `تاریخ تاسیس` | Incorporation date |
| 5 | `SBPlace` | `محل تاسیس` | Place of incorporation — **displayed, never saved** (§3.4) |
| 6 | `SCodeMelli` | `شناسه ملی` | Legal-entity national ID |
| 7 | `SCodePosti` | `کدپستی` | Postal code |
| 8 | `SAddress` | `آدرس` | Address |
| 12 | `SCodeSabt` | `کد ثبت` | Registration code |
| 13 | `SMaliatState` | `وضعیت مالیاتی` | Tax status |

Grid `G1` identical to §10.2 but sourced from the `SC_2` config set (§7.4b).
Minimum height 380 px, grid auto-sizes to `height − 275` (`CompanyEditU.pas:178-183`).

### 10.4 `PartyBankAccounts` (`SahamdarInfoU`)

* **Legacy:** `TSahamdarInfo`, caption `حسابهای بانکی اشخاص` = "Bank accounts of persons".
* **Entry:** `B_Bank` on the register (`SahamdarU.pas:236-241`), or `init_CodeStr` from cheque
  entry (`CheckEditAddU.pas:144`).

**Grid columns (`SahamdarInfoU.dfm:124-152`):**

| Field | Persian header | English | Width |
|---|---|---|---|
| `SI_St1` | `شماره کارت-شبا-حساب` | Card / IBAN / account number | 244 |
| `SI_St2` | `صاحب حساب` | Account holder | 150 |
| `SI_St3` | `نام بانک` | Bank name | 130 |
| `SI_St4` | `توضیحات` | Notes | 227 |

Multi-select is enabled (`dgMultiSelect`) and a checkbox column is shown
(`FixedColText.ShowCheckbox = True`), but nothing consumes the selection — cosmetic.
Footer sums are configured for `R_Bes`, `R_Bed`, `G_Bed`, `G_Bes`
(`SahamdarInfoU.dfm:116-120`) — **fields that do not exist in this query**; dead configuration
copy-pasted from `CardJariU`.

**Buttons:** `جدید` (New), `انتخاب` (Select — `B_SelectClick`, `:120-130`), `اصلاح` (Edit),
`حذف` (Delete, hidden), `حساب بانکی` (Bank account), `برگشت` (Back — `:114-118`).
**Only Select and Back are wired.** Double-click = Select (`SahamdarInfoU.dfm:100`).

**React notes:** implement full CRUD here (the legacy gap is a defect, not a design choice — see
§13-I5), and validate IBAN/card with the algorithms already present in `Dmu.pas:196-240`.

### 10.5 `AccountCodePicker` (`TarafU`) — a component, not a page

* **Legacy:** `TTaraf`, caption `طرف حساب` = "Counterparty".
* Rebuild as a **controlled React component** (`<AccountPicker value onChange />`) used by
  vouchers, cheques, rollover, settings, etc.

**Layout (`TarafU.dfm:20-274`):** four numeric inputs in one row, each with a `?` lookup button that
appears on focus (`TarafU.pas:264-269` etc.); four read-only name rows below, labelled
`کل:` (Kol), `معین:` (Moein), `تفضیل 1:` (Tafsil 1), `تفضیل 2:` (Tafsil 2) —
`TarafU.dfm:32,41,50,59`. Buttons `تایید` ("Confirm") and `برگشت` ("Back") — both currently just
close (`TarafU.dfm:151,136`).

**Required behaviours:** cascade reset on change; three-state colour feedback (§2.1); deeper level
disabled until parent resolves; leaf-only validity (`S_Child = 0`); `Enter` advances; digits only;
`?` opens the level-appropriate list.

**Public API to reproduce:** `value` (structured `{ko, mo, ta1, ta2}` **and** the dashed string),
`ssn`, `isValid`, `fullName`, `fullCodeName`, `lastName`.

### 10.6 `FiscalYearSwitcher` (`ChangesU`)

* **Legacy:** `TChangeS_F`, a grid over `Base` (`ChangesU.dfm:97` shows column `CO_ID`).
* Rebuild as a header dropdown listing `Co_Name — Co_Sub (FromDate … ToDate)`, with an
  **archived** badge when `IsActive <> 1`.
* Fix the Cancel-applies-the-change bug (`ChangesU.pas:78`).

### 10.7 `FiscalYearSettings` (`TanzimU`)

* **Legacy:** `TTanzimF` — one `TsEdit` + one pencil `TsSpeedButton` per field; each button opens a
  `GetString` prompt titled `تغيير اطلاعات` ("Change information") and posts **immediately**
  (`TanzimU.pas:160-280`, `:316-358`).
* Rebuild as a single form with one Save. Field labels and max lengths:

| Field | Persian prompt | English | Max | Line |
|---|---|---|---|---|
| `Co_Name` | `نام شرکت` | Company name | 100 | 164 |
| `Co_Address` | `آدرس شرکت` | Company address | 100 | 175 |
| `Co_Sub` | `نام سيستم` | System name / year label | 100 | 186 |
| `Co_Tel` | `تلفن` | Telephone | 20 | 208 |
| `Co_Fax` | `فاکس` | Fax | 20 | 197 |
| `Co_Web` | `وب سايت` | Website | 30 | 219 |
| `Co_EMail` | `پست الکترونيک` | E-mail | 30 | 240 |
| `Co_Sabt` | `شماره ثبت` | Registration number | 20 | 320 |
| `Co_Melli` | `شناسه ملی` | Legal-entity national ID | 20 | 342 |
| `Co_Egh` | `کد اقتصادي` | Economic code | 20 | 331 |
| `Co_Post` | `کد پستي` | Postal code | 20 | 353 |
| `FromDate` | `شروع سال مالي` | Fiscal-year start | 30, `Dm.IsDate` | 251-252 |
| `ToDate` | `پايان سال مالي` | Fiscal-year end | 30, `Dm.IsDate` | 263-264 |
| `BackupDir` | `مسير پشتيبان` | Backup path | 100 | 275 |
| `ARM` | *(file picker)* | Letterhead logo | image | 226-234 |

Plus the two account pointers `ENT1`/`ENT2` (→ `C1081`/`C1082`), each edited through the
`TarafU` picker and saved together by `B_Save` (`TanzimU.pas:282-305`).

### 10.8 `NewFiscalYear` (`MakeNewU`)

Fields: `COID` (new year id, pre-filled `current + 1`), `CoName`, `CoSuB`, `FromDate`, `ToDate`
(both pre-filled `+1` Jalali year), `Backup_Dir`. Buttons Save / Cancel.
Behaviour and messages in §1.5.

### 10.9 `YearEndRollover` (`EnteghalU`)

Two symmetrical panels — *current year* (closing) and *next year* (opening):

| Panel | Controls |
|---|---|
| Current | `Sal1` (read-only year label), `Sanad1` (closing voucher no.), `Date1` (Jalali), `Desc1` (description), `A_Code1` + `A_Code1Name` + `B_Code1` (closing control account via the picker) |
| Next | `Sal2`, `Sanad2`, `Date2`, `Desc2`, `A_Code2` + `A_Code2Name` + `B_Code2` |

Buttons `B_Save` / `B_Cancel`; a progress dialog (`WaitU`) sized to the number of accounts.
Full validation table and posting rules in §1.6.

### 10.10 `PartyCurrentAccount` (`CardJariU`) — party-linkage aspects only

> Grid internals, drill-downs and printing are documented by the reporting agent.

* **Entry:** toolbar button `خلاصه کارت` = "Card summary" (`Mainu.dfm:816-828`), permission **1131**
  (`Mainu.pas:920`), handler `Mainu.pas:443-446` → `CardJariF.init`.
* **Party-identity controls:**
  * `S_Card` (integer) — the party card number; a lookup button opens the register in picker mode
    (`CardJariU.pas:434-440`).
  * `COID` — fiscal-year dropdown bound to `Base` (`CardJariU.dfm:5927`, `:6541`); changing it
    re-runs the whole load (`CardJariU.pas:192-196`).
  * Group `sGroupBox2` — the **accounting** identity, read from `Sahamdar`:
    `S_Card`, `S_Name`, `S_Famil`, `S_Father`, `S_CMelli`, `S_Tel`.
  * Group `G_Saham` — the **share-register** identity, read from `Saham.Dbo.NSaham`:
    `N_Name`, `N_Famil`, `N_Father`, `N_CMelli`, `N_Tel`. Hidden entirely when the external database
    is absent (`CardJariU.pas:237-244`).
  * `S_Aks` — scanned ID image from `Saham_F` (`CardJariU.pas:329-337`).
  * `S_Rem` + `T_Rem` — net current-account balance and the `بدهکار` ("debtor") tag
    (`CardJariU.pas:350-360`).
* **React notes:** the two identity panels should be presented as *accounting record* vs
  *share-register record* with an explicit "out of sync" indicator, replacing the two magic strings
  at `CardJariU.pas:297` and `:321-322`.

### 10.11 Permission keys used by this domain

| Key | Persian caption | English | Guards |
|---|---|---|---|
| 1105 | `جاری اشخاص` | Person current accounts | Register menu item (`Mainu.pas:910`) |
| 1106 | `افزودن جاری جدید` | Add new current account | `B_New` (`SahamdarU.pas:107`) |
| 1107 | `اصلاح جاری` | Edit current account | `B_Edit`, `B_Bank` (`SahamdarU.pas:108,110`) |
| 1108 | `حذف جاری` | Delete current account | `B_Delete` (`SahamdarU.pas:109`) — dead |
| 1123 | `مشاهده دفتر معین` | View subsidiary ledger | `Report1` on the current-account card (`CardJariU.pas:259`) |
| 1131 | — | Card summary screen | `B_CardJari` (`Mainu.pas:920`) |
| *(admin)* | — | Lock/unlock, bypass locks | `Dm.Admin` (`SahamdarU.pas:112-113`, `Dmu.pas:923,970,985`) |

Permission test: `Dm.IsEnabel(UserID, Key)` (`Dmu.pas:145`).

---


---

[← Previous](07-09-sql-and-stored-procedures.md) · [Index](00-index.md) · [Next →](07-11-naming-map.md)
