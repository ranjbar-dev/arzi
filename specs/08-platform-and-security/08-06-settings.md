_Part of [08-platform-and-security](../08-platform-and-security.md) — [index](00-index.md)_

## 6. Settings

Settings live in **three** places with **no single owner**.

### 6.1 Store A — the ini file

Written through `TMyIni` (`INI.pas`), which is a thin wrapper over a
`TPropSaveFile` component `DM.PS` on the data module (`INI.pas:175-187`).

**File-path resolution is inconsistent** — a real bug:

| Consumer | Path formula | Line |
|---|---|---|
| `TMyIni.Create('')` (sets `IniFile`, but that field is **never read**) | `<exe>X.ini`, else `D:\Backup\Hesab.ini` | `INI.pas:241-251` |
| `DM.PS` (the store actually used by every read/write) | `ChangeFilePath(ChangeFileExt(ParamStr(0),'.ini'), 'D:\BACKUP')` = `D:\BACKUP\arzi.ini` | `Dmu.pas:711-713` |

So `TMyIni.IniFile` is dead code and every setting really goes to `D:\BACKUP\arzi.ini`.
The repo ships `arzi.ini` (empty), `arzi.ini1` and `arzi.local.ini` as samples.

**Encryption** (`INI.pas:43-170`): the classic Borland `InternalEncrypt` stream cipher
(constants `C1 = 52845`, `C2 = 22719`, default key `53269`) followed by a custom 3-byte→4-char
Base64-ish `PostProcess`/`Encode` using the alphabet
`A-Za-z0-9+/`. `Decrypt` is the inverse. ⛔ This is obfuscation, not encryption: the key
is a compile-time constant in the shipped binary.

`Utility.pas` carries a **byte-identical duplicate** of the same routines
(`Utility.pas:868-1016`) — `Util.Encrypt` and `MyIni`'s `Encrypt` produce the same output.

| Section | Key | Type | Default | What it changes | Read at | Written at |
|---|---|---|---|---|---|---|
| `Base` | `Program` | string | — | Vendor stamp `Green Gold`; **used as the import-file magic** (`InFile.pas:94-99` requires `GREENGOLD`) | `InFile.pas:94` | `Dmu.pas:715` |
| `Base` | `Programer` | string | — | Vendor stamp `Mohsen Ranjbar` | — | `Dmu.pas:716` |
| `Base` | `Mobile` | string | — | Vendor stamp `09131912805` | — | `Dmu.pas:717` |
| `Base` | `Contact` | string | — | Vendor stamp e-mail | — | `Dmu.pas:718` |
| `Base` | `GridFontSize` | int | `8` | Font size for every data grid (`DM.GridFontSize`) | `Dmu.pas:720` | *(never — read-only setting)* |
| `Base` | `CS1` | string `'0'`/`'1'` | absent | `'1'` ⇒ use the stored connection string; anything else ⇒ show the Data Link dialog | `Dmu.pas:724-725` | `Dmu.pas:736` |
| `Base` | **`CS2`** | **encrypted** string | `''` | **The full ADO/SQL Server connection string, including credentials** | `Dmu.pas:726` | `Dmu.pas:737` |
| `Base` | **`CS3`** | int | `0` | **The licence key** (see §7). `306239` = demo mode. | `Mainu.pas:892`, `testmainU.pas:225` | `testmainU.pas:61` |
| `Base` | `CS31` | int | — | Present in `arzi.local.ini` (`108763797`) but **never read or written by any code** — abandoned second key slot | — | — |
| `Base` | `SkinDirectory` | string | `<exe dir>\Skins\` | AlphaControls skin folder | `Mainu.pas:303` | `Mainu.pas:388` |
| `Base` | `SkinName` | string | `''` | Active skin (shipped default `Acryl (internal)`, `Mainu.dfm:10529`) | `Mainu.pas:305` | `Mainu.pas:389` |
| `Base` | `NewFactorCustomer` | int | — | Remembers the last counterparty used on a new warehouse invoice | *(warehouse module)* | *(warehouse module)* |
| `Base` | `NewFishBank` | int | — | Remembers the last bank used on a new deposit slip | *(treasury module)* | *(treasury module)* |
| `GetPass` | `ID` | int | current | **Pre-selected user id at next login** | `GetPassu.pas:124` | `GetPassu.pas:135` |
| `GetPass` | `COID` | int | current | Pre-selected company/fiscal year | `GetPassu.pas:125` | `GetPassu.pas:136` |
| *(per form)* | `Left`, `Top`, `Width`, `Height` | int | design-time | Window geometry, saved on close and restored on activate, for **~50 forms** | e.g. `TanzimU.pas:115-118` | e.g. `TanzimU.pas:308-311` |
| *(per form)* | `G1C`, `G1C0`, `G1C1`, `G2C`, `F0`–`F7` | int | — | Grid column widths | various | various |
| *(per form)* | `G1FontSize`, `F_Size`, `F_Type`, `F` | int | — | Per-grid font | various | various |
| *(per form)* | `A4orA5`, `C1`–`C4`, `L1`–`L3`, `T11`–`T13` | int | — | Per-report print options (paper size, which columns/labels to print) | various | various |
| *(per form)* | `M_RL`, `MRL` | int/string | — | Right-to-left / column-order toggle on list forms | various | various |
| *(per form)* | `D1`, `D2`, `Date1` | string | — | Last-used date range on report forms | various | various |
| *(per form)* | `TX0`, `TX1`, `TX2`, `state` | int | — | Last-used document-status filter | various | various |
| *(per form)* | `CD_Code`, `CM_Code`, `TM_Code`, `S_Bank`, `S_Bed`, `S_Bes` | int | — | Last-used account codes on treasury forms | various | various |

⛔ **`CS2` is the single worst finding in this domain.** It contains the SQL Server
connection string — server, database, user id and password — obfuscated with a
constant key that ships in the executable, in a world-readable file on a shared drive.
Every workstation therefore holds full DBA-ish credentials to the accounting database,
and the application's entire permission system is bypassable by anyone with a SQL
client. This must not survive into the rebuild in any form.

### 6.2 Store B — the `Base` table (per company / per fiscal year)

One row per fiscal year, keyed `Co_ID`. Edited by `TanzimF` (`TanzimU.pas`), reached
from `B_Setting` (perm 1109). Each field has its own pencil button that pops a
`GetString` prompt and writes immediately — there is no form-level Save for these
(only `C1081`/`C1082` go through `B_Save`, `TanzimU.pas:282-292`).

| Column | Persian label | English | Type | Edited at | Read at |
|---|---|---|---|---|---|
| `Co_ID` | *(read-only)* | Fiscal-year / company id | int | `MakeNewU.pas:119` only | everywhere (`DM.CO_ID`) |
| `Co_Name` | `نام شرکت` | Company name | varchar(100) | `TanzimU.pas:160-169` | `Mainu.pas:320`, `GetPassu.pas:102` |
| `Co_Sub` | `نام سيستم` | Sub-title / fiscal-year label | varchar(100) | `TanzimU.pas:182-191` | `Mainu.pas:321`, `GetPassu.pas:103` (`DM.RegSal`) |
| `Co_Address` | `آدرس شرکت` | Address | varchar(100) | `TanzimU.pas:171-180` | print layouts |
| `Co_Tel` | `تلفن` | Phone | varchar(20) | `TanzimU.pas:204-213` | print layouts |
| `Co_Fax` | `فکس` | Fax | varchar(20) | `TanzimU.pas:193-202` | print layouts |
| `Co_EMail` | `پست الکترونيکي` | E-mail | varchar(30) | `TanzimU.pas:236-245` | print layouts |
| `Co_Web` | `آدرس سايت` | Website | varchar(30) | `TanzimU.pas:215-224` | print layouts |
| `Co_Sabt` | `شماره ثبت` | Registration number | varchar(20) | `TanzimU.pas:316-325` | official invoice |
| `Co_Egh` | `شماره اقتصادي` | Economic code | varchar(20) | `TanzimU.pas:327-336` | official invoice |
| `Co_Melli` | `شناسه ملی` | National ID | varchar(20) | `TanzimU.pas:338-347` | official invoice |
| `Co_Post` | `کد پستي` | Postal code | varchar(20) | `TanzimU.pas:349-358` | official invoice |
| `ARM` | *(logo picker)* | Company logo (image blob) | image | `TanzimU.pas:226-234` | `Mainu.dfm:4384-4393` (`Arm1`), print layouts. **Explicitly skipped by the backup routine** (`Backup_U.pas:116`) |
| `FromDate` | `از تاريخ` | Fiscal year start (Jalali `YYYY/MM/DD`) | varchar(10) | `TanzimU.pas:247-257`, validated by `Dm.IsDate` | `Dmu.pas:1137-1142` (`From_Date`), used by `isValidDate` |
| `ToDate` | `تا تاريخ` | Fiscal year end | varchar(10) | `TanzimU.pas:259-269`, validated by `Dm.IsDate` | `Dmu.pas:1144-1149` (`To_Date`) |
| `No_Ko` | `تعداد ارقام` (Kol) | Digits in the *Kol* (top-level) account code | int | display only in `TanzimU`; no editor button | `Dmu.pas:1200` |
| `No_Mo` | *(same label)* | Digits in the *Moein* segment | int | display only | `Dmu.pas:1206` |
| `No_Ta1` | *(same label)* | Digits in the *Tafzil-1* segment | int | display only | `Dmu.pas:1213` |
| `No_Ta2` | *(same label)* | Digits in the *Tafzil-2* segment | int | display only | `Dmu.pas:1220` |
| `BackupDir` | `مسير پشتيبان` | Backup directory (a filesystem path) | varchar(100) | `TanzimU.pas:271-280` | `Mainu.pas:399` (auto `.bak`), `Backup_U.pas:44` (`.ABS`) |
| `C1081` | `اسناد نزد صندوق` | SSN of the "cheques on hand" account | int | `TanzimU.pas:285`, `:294-305` | `Dmu.pas:1073` (`SanDoogh_k`), `:1095` (`SanDoogh_M`) |
| `C1081C` | *(derived)* | Its display code string | varchar | `TanzimU.pas:287-288` | `Dmu.pas:1086` (`Sandoogh_KM`) |
| `C1082` | `اسناد در جریان وصول` | SSN of the "cheques in collection" account | int | `TanzimU.pas:286` | `Dmu.pas:1108` (`Jaryan_K`), `:1130` (`Jaryan_M`) |
| `C1082C` | *(derived)* | Its display code string | varchar | `TanzimU.pas:289-290` | `Dmu.pas:1121` (`Jaryan_KM`) |
| `IsActive` | *(no UI)* | `1` = year open, anything else = archived | int | **no editor anywhere in the codebase** | `Dmu.pas:1008-1013` — blocks all new document creation |
| `Real_Len` | *(commented out)* | Decimal places | int | — | `TanzimU.pas:135` *(commented)* |

⛔ `IsActive` gates every write to a fiscal year but has **no UI**. Today it can only be
changed with a SQL client. The rebuild needs a proper open/close-period feature.

### 6.3 Store C — the `Tanzim` table (print/document parameters)

`DM.Tanzim : TADOTable`, `TableName = 'Tanzim'` (`Dmu.dfm:744-750`).
Schema: `T_ID` (int, key), `T_Str` (varchar), `T_Int` (int), `T_Desc` (varchar).

Accessors `TDM.Get_paramstr(id)` (`Dmu.pas:469-499`) and `TDM.Set_paramstr(id, s)`
(`Dmu.pas:501-508`). `Get_paramstr` **auto-creates** a missing row, seeding `T_Str` and
`T_Desc` with a hard-coded Persian label and `T_Int = '0'`. `Set_paramstr` silently
does nothing if the row does not exist.

Edited by `TanzimChap` (`TanzimChapu.pas`), ribbon button `تنظیمات فرمها`
(`Mainu.pas:506-509`) — **not permission-gated**.

| `T_ID` | Seed label (Persian) | English | Type | Default | Effect | Editor | Seed |
|---|---|---|---|---|---|---|---|
| 1001 | `فاکتور امضا 1` | Invoice signature 1 | string | label text | Signature block on printed invoices | `S1001` (`TanzimChapu.pas:68`, `:103`) | `Dmu.pas:472` |
| 1002 | `فاکتور امضا 2` | Invoice signature 2 | string | label text | ″ | `S1002` | `Dmu.pas:473` |
| 1003 | `فاکتور امضا 3` | Invoice signature 3 | string | label text | ″ | `S1003` | `Dmu.pas:474` |
| 1004 | `فاکتور امضا 4` | Invoice signature 4 | string | label text | ″ | `S1004` | `Dmu.pas:475` |
| 1005 | `فاکتور عنوان 1` | Invoice heading 1 | string | label text | Invoice header line | `S1005` | `Dmu.pas:476` |
| 1006 | `فاکتور عنوان 2` | Invoice heading 2 | string | label text | ″ | `S1006` | `Dmu.pas:477` |
| 1007 | `طرف حساب` | Counterparty | string | label text | Default counterparty caption | `S1007` | `Dmu.pas:478` |
| 1008 | `نمایش مبلغ` | Show amount | bool as `'0'`/`'1'` | `'0'` | Show the amount column on printed invoices | `S1008` checkbox (`TanzimChapu.pas:81`, `:111`) | `Dmu.pas:479` |
| 1009 | `نمایش تخفیف` | Show discount | bool | `'0'` | Show the discount column | `S1009` | `Dmu.pas:480` |
| 1010 | `نمایش مالیات` | Show VAT | bool | `'0'` | Show the VAT column | `S1010` | `Dmu.pas:481` |
| 1011 | `سند امضا 1` | Document signature 1 | string | label text | Signature block on printed accounting documents | `S1011` | `Dmu.pas:482` |
| 1012 | `سند امضا 2` | Document signature 2 | string | label text | ″ | `S1012` | `Dmu.pas:483` |
| 1013 | `سند امضا 3` | Document signature 3 | string | label text | ″ | `S1013` | `Dmu.pas:484` |
| 1014 | `سند امضا 4` | Document signature 4 | string | label text | ″ | `S1014` | `Dmu.pas:485` |
| 1015 | `پانویس فاکتور رسمی` | Official-invoice footer | multi-line string | label text | Footer text on the VAT invoice | `Tanzim1015` memo | `Dmu.pas:486` |

Note `T_Int` is written once (as the string `'0'`) and **never read**.
`Tanzim` is **not** keyed by `Co_ID` — these settings are global across all fiscal years.

### 6.4 Store D — the Windows registry (dead)

`TDM.GetReg_String` / `SetReg_String` (`Dmu.pas:545-565`) and the identical
`TSysInfo.GetReg_String` / `SetReg_String` (`LockUnit.pas:39-59`) read/write
`HKEY_LOCAL_MACHINE\Software\<PrgName>`. The only call sites (skin directory/name)
are **commented out** (`Mainu.pas:299-300`, `Mainu.pas:385-386`) — the ini file
replaced them. `Utility.pas:342-368` has a third copy. The registry is **not** used at
runtime, except by the licence fingerprint (§7), which *reads* hardware keys under
`HKLM\HARDWARE\DESCRIPTION\SYSTEM`.

### 6.5 Hard-coded values that behave like settings

| Value | Where | Why it matters |
|---|---|---|
| `D:\BACKUP` | `Dmu.pas:711` | Settings-file directory. Fails on any machine without a `D:` drive. |
| `D:\Backup\Hesab.ini` | `INI.pas:246` | Fallback settings path. |
| `\\pesteh\SahamData\` | `Dmu.pas:759` | UNC share for the shareholder module. |
| `Saham.Dbo`, `Anbar.Dbo`, `Rppc_Solution.Dbo` | `Dmu.pas:758-773` | Sibling databases; presence toggles whole feature areas. |
| `306239` | `Mainu.pas:895` | Demo-mode licence key; caps the ledger at 200 rows. |
| `233576133` | `testmainU.pas:231` | Master licence bypass hash (see §7). |
| `234384` | `Lab.pas:125` | PIN for a lab/weighbridge reprint. |
| `'Mohsen' + '68411' + '211'` | `Backup_U.pas:141` | Backup-archive password. |
| `'d+B6Y52L6r0dU2UPhjhf'` | `testmainU.pas:93` | Encrypted developer password for the licence generator. |
| `53269`, `52845`, `22719` | `INI.pas:15-16,45-46`; `Utility.pas:1005-1013` | Cipher key and constants for `CS2`. |

---


---

Prev: [5. Audit trail / change log](08-05-audit-trail-change-log.md) · Next: [7. Licensing / copy protection](08-07-licensing-copy-protection.md)
