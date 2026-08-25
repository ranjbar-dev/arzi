_Part of [08-platform-and-security](../08-platform-and-security.md) — [index](00-index.md)_

## 4. Authorization

### 4.1 The model

Two orthogonal mechanisms, and nothing else:

1. **`Password.Supervisor = 1`** — a boolean super-admin. Checked directly in five
   places (`Mainu.pas:957-961`) and cached as `DM.Admin` at login (`GetPassu.pas:97`).
   `DM.Admin` short-circuits three document-lock predicates
   (`Dmu.pas:923-924`, `:970-971`, `:985-986`), i.e. **a supervisor can edit locked
   documents, locked accounts and locked current accounts.**
2. **A grant table `Pass_Config`** — pure allow-list, no roles, no groups, no
   inheritance, no deny.

### 4.2 `Pass_Config` schema and the runtime check

Written by `TAdminF.B_SaveClick` (`Admin.pas:185-218`):

```
Delete Pass_Config Where P_User = :UID
Insert Pass_Config (P_User, P_ID, P_DESC) Values (:User, :ID, :Desc)
```

| Column | Meaning |
|---|---|
| `P_User` | `Password.UserCode` |
| `P_ID` | numeric permission id (1100–2125) |
| `P_DESC` | the Persian **caption of the checkbox**, denormalised into every row |

Save is **delete-all-then-reinsert-granted** and is **not wrapped in a transaction**
(`Admin.pas:192-214`). A crash or connection drop between the `Delete` and the last
`Insert` leaves the user with a partial or empty permission set. ⛔ DO NOT PORT.

The runtime check, `TDM.IsEnabel(UserID, Key): Boolean` (`Dmu.pas:1552-1562`):

```sql
Select * From Pass_Config where P_User = <UserID> and P_ID = <Key>
```

`Result := Q1.RecordCount > 0`. **One database round-trip per permission, per check.**
`TMain.Reload` therefore issues ~35 queries on every login and after every admin visit
(`Mainu.pas:907-953`). Note also the misspelling — the function is `IsEnabel`, not
`IsEnabled`.

⚠️ **The check is presentation-only.** With three exceptions (`SanadEditU.pas:816`,
`TasfiehFactor.pas:227/236/264/273`) every call site assigns to `.Enabled` or
`.Visible` on a VCL control. There is **no authorization at the data layer**. Any code
path that reaches a form by another route — or a user with a SQL client, since the app
connects with a single shared SQL login stored in `CS2` — is completely unconstrained.
⛔ In the Rust rebuild authorization must be enforced server-side on every command and
query, never in the React layer.

### 4.3 Admin UI (`Admin.pas` + `Admin.dfm`)

Form `AdminF`, reached from `B_Admin`, gated on `Supervisor = 1` (`Mainu.pas:957`).

- Left: `G1: TsDBGrid` bound to `DM.Password` showing `UserCode` (`کد`), `UserName`,
  `Password` (`Visible = False`), `Enabled`, `Supervisor` (title `Admin`)
  (`Admin.dfm:498-558`). Row-select mode (`dgRowSelect`).
- Right: four `TsPanel` columns of `TsRollOutPanel` groups, each holding
  `TsCheckBox` controls **named `C<permission-id>`**.
- `ReLoad` (`Admin.pas:220-229`) loops `i := 1000 to 3000`, does
  `FindComponent('C' + IntToStr(i))` and sets `.Checked := Dm.IsEnabel(UserID, i)`.
  ⚠️ **That is 2001 iterations × one SQL query for each of the ~85 that resolve**, run on
  *every* grid row change (`DS1DataChange`, `Admin.pas:231-234`).
- `B_SaveClick` saves the **currently selected user only**, using the same
  `FindComponent` sweep (`Admin.pas:204-214`).
- Context menu `PopupMenu1` (`Admin.dfm:1287-1315`):

| Item | Caption | Action | Line |
|---|---|---|---|
| `E_User` | `Enable/Disable User` | Toggles `Enabled` 0↔1 | `Admin.pas:257-267` |
| `N_User` | `New User` | Prompts for name (max 20), assigns `max+1`, `Enabled=1`, `Supervisor=0`, **no password set** | `Admin.pas:165-183` |
| `C_Pass` | `Change Password` | Prompts via unmasked `GetString`, writes plaintext | `Admin.pas:281-292` |
| `C_Name` | `Change UserName` | Prompts, writes | `Admin.pas:269-279` |

⛔ There is **no delete-user**, **no way to grant/revoke `Supervisor` from the UI**
(the grid column is read-only in practice — `dgRowSelect` prevents cell editing), and
**a new user is created with an empty password and is immediately loginable** (the login
query filters on `Enabled = 1` only, and `''` trims/uppercases to `''` which matches an
empty typed password). This is a live account-takeover path.

### 4.4 Full permission matrix

`Perm` = `Pass_Config.P_ID`. `Admin UI` = the `Admin.dfm` line of the checkbox caption.
`Enforced at` cites every runtime check found across the codebase.

#### Group `تنظیمات حسابداری` — Accounting setup (`sRollOutPanel1`, `Admin.dfm:35-152`)

| Perm | Persian caption | English | What it allows | Admin UI | Enforced at |
|---|---|---|---|---|---|
| 1100 | `سرفصلهاي حسابداري` | Chart of accounts | Enables the whole `Sarfasl_H` ribbon button | `Admin.dfm:51` | `Mainu.pas:907` |
| 1101 | `ليست سرفصلها` | Account list | Menu item → `SNew` (account browser/editor) | `Admin.dfm:63` | `Mainu.pas:908`, `ListSarfaslu.pas:78` |
| 1102 | `ايجاد سرفصل` | Create account | Nominally the "add account" item — **hard-disabled** in `Reload` and the handler `exit`s immediately | `Admin.dfm:75` | `Mainu.pas:909` (forced `False`), `ListSarfaslu.pas:78` |
| 1103 | `اصلاح سرفصل` | Amend account | *(assigned to `S1.Enabled` then immediately overwritten by 1104 — dead)* | `Admin.dfm:87` | `ListSarfaslu.pas:79` |
| 1104 | `حذف سرفصل` | Delete account | *(same `S1.Enabled`, last writer wins)* | `Admin.dfm:99` | `ListSarfaslu.pas:80` |
| 1105 | `جاري اشخاص` | Personal current accounts | `Jari_List` menu item → `Sahamdar` | `Admin.dfm:111` | `Mainu.pas:910` |
| 1106 | `افزودن جاري جديد` | Add current account | `B_New` on the `Sahamdar` form | `Admin.dfm:123` | `SahamdarU.pas:107` |
| 1107 | `اصلاح جاري` | Amend current account | `B_Edit` **and** `B_Bank` on `Sahamdar` | `Admin.dfm:135` | `SahamdarU.pas:108`, `SahamdarU.pas:110` |
| 1108 | `حذف جاري` | Delete current account | `B_Delete` on `Sahamdar`. Checkbox is `Visible = False` — **the permission can never be granted through the UI** | `Admin.dfm:147-150` | `SahamdarU.pas:109` |

> ⚠️ `ListSarfaslu.pas:78-80` assigns 1102, 1103 and 1104 to the *same* control `S1`
> on consecutive lines. Only 1104 takes effect. Treat 1102/1103 as unimplemented.

#### Group `صدور سند معین` — Subsidiary document issuance (`sRollOutPanel2`, `Admin.dfm:153-372`)

| Perm | Persian caption | English | What it allows | Admin UI | Enforced at |
|---|---|---|---|---|---|
| 1112 | `اسناد معین` | Subsidiary documents | Enables the `Asnad_Moein` ribbon button (gateway to `POP_Moein`) | `Admin.dfm:169` | `Mainu.pas:912` |
| 1113 | `ثبت سند معين` | Post subsidiary document | `SMoein0` menu item; `B_New` on the document browser | `Admin.dfm:217` | `Mainu.pas:916`, `SanadViewU.pas:744` |
| 1114 | `اصلاح سند معين` | Amend document | `B_Edit` on browser; **hard gate** inside the editor | `Admin.dfm:229` | `SanadViewU.pas:745`, **`SanadEditU.pas:816`** |
| 1115 | `حذف سند معين` | Delete document | `B_Delete` on browser | `Admin.dfm:241` | `SanadViewU.pas:746` |
| 1116 | `تاييد سند معين` | Approve document (TX 0→1) | `B_TX01`, only when `_TX = 0` | `Admin.dfm:253` | `SanadViewU.pas:749` |
| 1117 | `ثبت دائم سند` | Permanently post (TX 1→2) | `B_TX12`, only when `_TX = 1` | `Admin.dfm:265` | `SanadViewU.pas:751` |
| 1118 | `برگشت به تحرير` | Revert to draft (TX 1→0) | `B_TX10`, only when `_TX = 1` | `Admin.dfm:277` | `SanadViewU.pas:750` |
| 1119 | `تغيير تاريخ سند` | Change document date | `B_ChangeDate`, only when `_TX = 0` | `Admin.dfm:289` | `SanadViewU.pas:747` |
| 1120 | `تغيير شماره سند` | Change document number | `B_ChangeNo`, only when `_TX = 0` | `Admin.dfm:302` | `SanadViewU.pas:748` |
| 1121 | `چاپ سند معين` | Print document | `SMoein4` menu item; `B_ViewPrint`; `B_ViewSanad` on cheque and petty-cash lists | `Admin.dfm:315` | `Mainu.pas:917`, `SanadViewU.pas:753`, `CheckListU.pas:124`, `TankhahList.pas:98` |
| 1125 | `ليست اسناد تاييد شده` | List approved documents | `SMoein2` → `ViewAsnad(1)` | `Admin.dfm:193` | `Mainu.pas:914` |
| 1126 | `ليست اسناد ثبت شده` | List posted documents | `SMoein3` → `ViewAsnad(2)` | `Admin.dfm:205` | `Mainu.pas:915` |
| 1127 | `ليست اسناد درحال تحرير` | List draft documents | `SMoein1` → `ViewAsnad(0)` | `Admin.dfm:181` | `Mainu.pas:913` |
| 1142 | `کپی سند معین` | Copy document | `B_Copy` on browser. *(The `SMoein7` ribbon copy item is **not** gated.)* | `Admin.dfm:328` | `SanadViewU.pas:755` |
| 1143 | `ادغام اسناد` | Merge documents | `B_Merge` on browser | `Admin.dfm:340` | `SanadViewU.pas:756` |
| 1144 | `قفل سند` | Lock document | `B_Lock` on browser (opens lock/unlock popup) | `Admin.dfm:353` | `SanadViewU.pas:757` |
| 1145 | `برگشت از ثبت دائم` | Revert permanent posting (TX 2→1) | `B_TX21`, only when `_TX = 2` | `Admin.dfm:366` | `SanadViewU.pas:752` |

#### Group `گزارش حسابداری` — Accounting reports (`sRollOutPanel5`, `Admin.dfm:853-947`)

| Perm | Persian caption | English | What it allows | Admin UI | Enforced at |
|---|---|---|---|---|---|
| 1122 | `کزارش حسابداري` *(sic — `کزارش` is a typo for `گزارش`)* | Accounting reports | The whole `Report` ribbon dropdown | `Admin.dfm:871` | `Mainu.pas:927` |
| 1123 | `مشاهده دفتر معين` | View subsidiary ledger | `Report1` menu item; `Report1` on the current-account card | `Admin.dfm:883` | `Mainu.pas:921`, `CardJariU.pas:259` |
| 1124 | `تراز آزمايشي 4 ستوني` | 4-column trial balance | `Report2` menu item | `Admin.dfm:895` | `Mainu.pas:922` |
| 1128 | `خلاصه اسناد معين` | Document summary | `SMoein6` → `MoeinZip` | `Admin.dfm:907` | `Mainu.pas:918` |
| 1129 | `خلاصه گردش اسناد` | Document turnover summary | **Never checked anywhere** — orphan | `Admin.dfm:919` | *(none)* |
| 1130 | `تبدیل اسناد معین به روزنامه` | Convert subsidiary→journal | **Never checked anywhere** — orphan; the checkbox also sits outside every roll-out panel (`Admin.dfm:20-27`) so it renders loose on the form | `Admin.dfm:25` | *(none)* |
| 1131 | `خلاصه کارت` | Card summary | `B_CardJari` ribbon button | `Admin.dfm:931` | `Mainu.pas:920` |
| 1141 | `دفتر کل` | General ledger | `Report9` menu item → `DKolF` | `Admin.dfm:943` | `Mainu.pas:923` |

#### Group `انبار` — Warehouse (`sRollOutPanel4`, `Admin.dfm:681-797`)

| Perm | Persian caption | English | What it allows | Admin UI | Enforced at |
|---|---|---|---|---|---|
| 1401 | `تنظيمات انبار` | Warehouse settings | `Anbar_Tanzim` ribbon button | `Admin.dfm:697` | `Mainu.pas:929` |
| 1402 | `معرفي اجناس` | Item master | `Anbar_Ajnas` ribbon button | `Admin.dfm:709` | `Mainu.pas:930` |
| 1403 | `صدور فاکتور` | Issue invoice | `Anbar_Factor` ribbon button (gateway to the four kinds) | `Admin.dfm:781` | `Mainu.pas:931` |
| 1404 | `صدور فاکتور خريد` | Issue purchase invoice | `Anbar1` (`صدور رسید انبار`, kind 1); `T1` on the invoice list | `Admin.dfm:721` | `Mainu.pas:932`, `AnbarListU.pas:151` |
| 1405 | `صدور فاکتور فروش` | Issue sales invoice | `Anbar2` (`صدور حواله انبار`, kind 2); `T2` | `Admin.dfm:745` | `Mainu.pas:933`, `AnbarListU.pas:152` |
| 1406 | `فاکتور برگشت از فروش` | Sales-return invoice | `Anbar4` (kind 4); `T4` | `Admin.dfm:733` | `Mainu.pas:934`, `AnbarListU.pas:153` |
| 1407 | `فاکتور برگشت از خريد` | Purchase-return invoice | `Anbar3` (kind 3); `T3` | `Admin.dfm:757` | `Mainu.pas:935`, `AnbarListU.pas:154` |
| 1408 | `اصلاح فاکتور` | Amend invoice | `Anbar0` menu item; `Anbar0` on the invoice list | `Admin.dfm:769` | `Mainu.pas:936`, `AnbarListU.pas:155` |
| 1414 | `حذف فاکتور` | Delete invoice | `AR_Delete` on the invoice list | `Admin.dfm:793` | `AnbarListU.pas:158` |

#### Group `گزارش انبار` — Warehouse reports (`sRollOutPanel3`, `Admin.dfm:586-680`)

| Perm | Persian caption | English | What it allows | Admin UI | Enforced at |
|---|---|---|---|---|---|
| 1409 | `گزارش انبار` | Warehouse report | `Anbar_Report` ribbon button | `Admin.dfm:651` | `Mainu.pas:938` |
| 1410 | `چاپ فاکتور` | Print invoice | `AR_Print1` **and** `AR_Print2` (both share this id); `AR_Chap` on the invoice list | `Admin.dfm:603` | `Mainu.pas:939-940`, `AnbarListU.pas:156` |
| 1411 | `گزارش عملکرد انبار` | Warehouse performance report | `AR_Kholaseh` menu item — **but that item has no `OnClick`**, so the grant is inert | `Admin.dfm:615` | `Mainu.pas:941` |
| 1412 | `کارت جنسي` | Stock card | `AR_Jensi` ribbon button | `Admin.dfm:627` | `Mainu.pas:942` |
| 1413 | `گزارش موجودی انبار` | Inventory balance report | `AR_Amalkard` ribbon button → `Anbar_MandehF` | `Admin.dfm:639` | `Mainu.pas:943` |
| 1415 | `Save To Disk` | Save to disk | **Never checked anywhere** — orphan | `Admin.dfm:663` | *(none)* |
| 1416 | `گزارش عملکرد انبار` | Warehouse performance report (dup) | Checkbox `Visible = False`; the only call site is commented out | `Admin.dfm:675-678` | `Mainu.pas:944` *(commented)* |

#### Group `اسناد دریافتی` — Receivable instruments (`sRollOutPanel10`, `Admin.dfm:1037-1153`)

| Perm | Persian caption | English | What it allows | Admin UI | Enforced at |
|---|---|---|---|---|---|
| 2101 | `لیست اسناد دریافتی` | Receivable-instrument list | `B_DaryaftCheck` ribbon button; also OR-gates `AR_Variz` on the invoice list | `Admin.dfm:1053` | `Mainu.pas:950`, `AnbarListU.pas:503` |
| 2102 | `دریافت چک` | Receive cheque | `S_New` on the receivables list; `F_New` on the deposit-slip list (second `Reload`); `B_AddC` on invoice settlement | `Admin.dfm:1065` | `CheckListDU.pas:239`, `FishListD.pas:165`, `TasfiehFactor.pas:91` |
| 2103 | `اصلاح چک` | Amend cheque | `S_Edit`; `F_Edit`; part of `B_Edit`; **hard gate** in settlement | `Admin.dfm:1077` | `CheckListDU.pas:240`, `FishListD.pas:166`, `TasfiehFactor.pas:94`, **`TasfiehFactor.pas:273`** |
| 2104 | `حذف چک` | Delete cheque | `S_Delete`; `F_Delete`; **hard gate** in settlement | `Admin.dfm:1089` | `CheckListDU.pas:241`, `FishListD.pas:167`, **`TasfiehFactor.pas:236`** |
| 2105 | `واگذار به بانک` | Deposit to bank | `S_Bank` | `Admin.dfm:1101` | `CheckListDU.pas:242` |
| 2106 | `برگشت از بانک` | Return from bank | `S_BBank` | `Admin.dfm:1113` | `CheckListDU.pas:243` |
| 2107 | `وصول چک` | Collect cheque | `S_Vosool` | `Admin.dfm:1125` | `CheckListDU.pas:244` |
| 2108 | `حذف وصول چک` | Delete collection | `S_DVosool` | `Admin.dfm:1137` | `CheckListDU.pas:245` |
| 2109 | `برگشت به صاحب` | Return to owner | `S_Bargasht` | `Admin.dfm:1149` | `CheckListDU.pas:246` |

#### Group `صدور چک` — Cheque issuance (`sRollOutPanel7`, `Admin.dfm:956-1036`)

| Perm | Persian caption | English | What it allows | Admin UI | Enforced at |
|---|---|---|---|---|---|
| 2110 | `لیست صدور چک` | Cheque-issuance list | `B_SodoorCheck` ribbon button | `Admin.dfm:972` | `Mainu.pas:951` |
| 2111 | `صدور چک` | Issue cheque | `B_New` | `Admin.dfm:984` | `CheckListU.pas:118` |
| 2112 | `اصلاح چک` | Amend cheque | `B_Edit` | `Admin.dfm:996` | `CheckListU.pas:119` |
| 2113 | `نمایش` | View | `B_View` | `Admin.dfm:1008` | `CheckListU.pas:121` |
| 2114 | `چاپ` | Print | `B_Print1`/`B_Print2` on the cheque list, the cheque editor (twice) and the petty-cash editor | `Admin.dfm:1020` | `CheckListU.pas:122-123`, `CheckEditU.pas:266-267`, `CheckEditU.pas:525-526`, `TankhahEdit.pas:246-247`, `TankhahEdit.pas:503-504` |
| 2125 | `حذف چک` | Delete cheque | `B_Delete` | `Admin.dfm:1032` | `CheckListU.pas:120` |

#### Group `واریز نقدی و کارتخوان` — Cash deposit & card reader (`sRollOutPanel11`, `Admin.dfm:1154-1210`)

| Perm | Persian caption | English | What it allows | Admin UI | Enforced at |
|---|---|---|---|---|---|
| 2115 | `واریز نقدی` | Cash deposit | `B_Variz` ribbon button; OR-gates `AR_Variz` | `Admin.dfm:1170` | `Mainu.pas:952`, `AnbarListU.pas:503` |
| 2116 | `فیش جدید` | New deposit slip | `F_New`; `B_AddF` on invoice settlement | `Admin.dfm:1182` | `FishListD.pas:139`, `TasfiehFactor.pas:90` |
| 2117 | `اصلاح فیش` | Amend deposit slip | `F_Edit`; part of `B_Edit`; **hard gate** in settlement | `Admin.dfm:1194` | `FishListD.pas:140`, `TasfiehFactor.pas:93`, **`TasfiehFactor.pas:264`** |
| 2118 | `حذف فیش` | Delete deposit slip | `F_Delete`; `B_Delete` in settlement; **hard gate** | `Admin.dfm:1206` | `FishListD.pas:141`, `TasfiehFactor.pas:96`, **`TasfiehFactor.pas:227`** |

> ⚠️ `FishListD.pas` has **two** `Reload`-style routines. Lines 139-141 map
> `F_New/F_Edit/F_Delete` → 2116/2117/2118; lines 165-167 map the *same three controls*
> → 2102/2103/2104. Which wins depends on which routine ran last. This is a genuine
> ambiguity — resolve it with the customer before rebuilding. See §13.

#### Group `تنخواه` — Petty cash (`sRollOutPanel13`, `Admin.dfm:1211-1280`)

| Perm | Persian caption | English | What it allows | Admin UI | Enforced at |
|---|---|---|---|---|---|
| 2120 | `لیست تنخواه گردان` | Petty-cash float list | `B_Tankhah` ribbon button | `Admin.dfm:1227` | `Mainu.pas:953` |
| 2121 | `لیست جدید` | New list | `B_New` | `Admin.dfm:1239` | `TankhahList.pas:92` |
| 2122 | `اصلاح لیست` | Amend list | `B_Edit` | `Admin.dfm:1251` | `TankhahList.pas:93` |
| 2123 | `نمایش` | View | `B_View` | `Admin.dfm:1263` | `TankhahList.pas:94` |
| 2124 | `چاپ` | Print | `B_Print1`/`B_Print2` | `Admin.dfm:1275` | `TankhahList.pas:95-96` |

#### Group `متفرقه` — Miscellaneous (`sRollOutPanel14`, `Admin.dfm:798-842`)

| Perm | Persian caption | English | What it allows | Admin UI | Enforced at |
|---|---|---|---|---|---|
| 1109 | `تنظيمات برنامه` | Application settings | `B_Setting` ribbon button → `TanzimF` | `Admin.dfm:814` | `Mainu.pas:946` |
| 1110 | `ايجاد پشتيبان` | Create backup | `B_Backup` ribbon button → `BackupForm` | `Admin.dfm:826` | `Mainu.pas:947` |
| 1111 | `تغيير پوسته` | Change skin | `B_Skin` ribbon button | `Admin.dfm:838` | `Mainu.pas:948` |

#### Supervisor-only (no `Pass_Config` id)

| Control | What it allows | Enforced at |
|---|---|---|
| `B_Admin` | Open the user & permission admin form | `Mainu.pas:957` |
| `B_Enteghal1` | Year-end account closing (`NewFinalF`) | `Mainu.pas:958` |
| `B_Enteghal2` | Inventory-balance transfer (hidden) | `Mainu.pas:959` |
| `B_Enteghal3` | Balance carry-forward (`EnteghalF`) | `Mainu.pas:960` |
| `DM.Admin` short-circuit | Edit **locked** documents / accounts / current accounts | `Dmu.pas:923-924`, `Dmu.pas:970-971`, `Dmu.pas:985-986` |

#### Permission ids that exist but are never granted or never checked

| Perm | Status |
|---|---|
| 1108 | Checkbox `Visible = False` → **ungrantable**, but *is* checked (`SahamdarU.pas:109`). Effectively "delete current account" is disabled for all non-supervisors, forever. |
| 1129, 1130, 1415 | Checkbox exists, **no call site**. Dead grants. |
| 1416 | Checkbox hidden **and** call site commented out. |
| 1102, 1103 | Checked but the assignment is immediately overwritten (`ListSarfaslu.pas:78-80`) / hard-disabled (`Mainu.pas:909`). |
| 1411 | Checked, but the menu item it enables has no handler. |
| **2119** | **Gap in the numbering.** No checkbox, no call site. |
| 1201, 1209, 1212–1216 | Appear only as menu-item `Tag` values in `Mainu.dfm`; **never** passed to `IsEnabel`. Vestigial from an earlier design. |

---


---

Prev: [3. Authentication](08-03-authentication.md) · Next: [5. Audit trail / change log](08-05-audit-trail-change-log.md)
