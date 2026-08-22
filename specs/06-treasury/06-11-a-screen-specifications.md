_Part of [06-treasury](../06-treasury.md) — [index](00-index.md)_

## 11. Screen specifications

All forms are right-to-left (`BiDiMode = bdRightToLeft`) and all persist their size, position, grid
column widths and grid font size to the per-user INI file on `FormClose`, restoring them on
`FormActivate`. The React rebuild should keep column-width persistence (users rely on it) but move it
to server-side user preferences.

Every account-code field follows the same three-control idiom, which the rebuild should collapse into
one component:

> a code `TsEdit` (e.g. `S_Bes`) whose `.Tag` holds the resolved `Sarfasl.S_SSN` (`0` = unresolved),
> a read-only full-path name `TsEdit` (`S_BesName`), a read-only short name `TsEdit` (`S_BesN`), and
> a `...` `TsSpeedButton` opening the `Taraf` 4-segment picker modally. `OnChange` re-resolves the
> code on every keystroke via `Taraf.Set_FullCode` / `Taraf.Get_Valid` / `Taraf.Get_SSn`.
> Several of these fields carry a right-click popup `ذخیره به عنوان پیش فرض` "save as default" that
> writes the current code to the INI.

Navigation map:

```
Main menu ─┬─ "received cheques"  → CheckListDU ─┬─ CheckDaryaftU   (new / edit)
           │                                     ├─ CheckDaryaft2U  (deposit)
           │                                     ├─ CheckBargashtu  (bounce)
           │                                     ├─ CheckVosoolU    (collect)
           │                                     └─ CheckEsterdadU  (return)
           ├─ "deposit slips"     → FishListD    → FISHDaryaftU     (new / edit)
           ├─ "issued cheques"    → CheckListU   → CheckEditU       → CheckEditAddU (line)
           │                                                        → SanadEditF.View (voucher)
           └─ "petty cash"        → TankhahList  → TankhahEdit      → TankhahEditAddu (line)
                                                                    → SanadEditF.View (voucher)
```

All child forms are **modal** (`ShowModal`) and communicate results back through the form's `Tag`
property (`Tag = 0` means "cancelled / nothing saved"; otherwise it holds the new row id).

---

### 11.1 `CheckListDU` — Received-cheque list

**Purpose**: the hub for received cheques. Browse, and launch every lifecycle transition.
**Window title**: `   تنظیمات بانک` "bank settings" — **wrong**, copy-pasted from `BankTanzim`
(`CheckListDU.dfm:5`). Same mistake on `CheckListU.dfm:5`.
**Entry**: `Mainu.pas:524` → `CheckListDF.init`.

**Layout**: master grid `G1` (top, `alClient`), splitter, detail grid `G2` (bottom, the `DCheck2`
history of the selected cheque), a toolbar strip `sPanel3` (top) and a button bar `sPanel1`.

**Master grid `G1`** — bound to `Q1` (`Select * From DCheck … Order By S_Dates`):

| # | Field | Persian header | English header |
|---|---|---|---|
| 1 | `S_StateName` | `وضعیت` | Status |
| 2 | `S_Sanad` | ` سند` | Voucher |
| 3 | `S_CheckNo` | `شماره چک` | Cheque number |
| 4 | `S_Date` | `تاریخ دریافت` | Received on |
| 5 | `S_Mab` | `مبلغ چک` | Amount |
| 6 | `S_DateS` | `سررسید` | Due date |
| 7 | `S_BesCR` | `دهنده چک` | Cheque giver (account code) |
| 8 | `S_BesName` | `نام صاحب حساب` | Account holder name |
| 9 | `S_Desc` | `شرح` | Description |
| 10 | `S_linkPrg` | `اطلاعات اضافی` | Extra information — rendered by `Q1S_linkPrgGetText` as `' فاکتور ' + S_LinkSSN` when `= 1`, blank otherwise (`CheckListDU.pas:279-293`) |

**Detail grid `G2`** — bound to `Q2` (`Select * From DCheck2 Where S_Link=… Order By S_SSN`),
refreshed by `DS1DataChange` (`CheckListDU.pas:156-166`):

| # | Field | Persian header | English header |
|---|---|---|---|
| 1 | `S_COID` | `سال` | Year |
| 2 | `S_Sanad` | `سند` | Voucher |
| 3 | `S_Mab` | `مبلغ` | Amount (formatted by `Q2S_MabGetText`) |
| 4 | `S_Date` | `تاریخ تغییر وضعیت` | Status-change date |
| 5 | `S_StateName` | `وضعیت` | Status |
| 6 | `S_Desc` | `شرح` | Description |
| 7 | `S_UserID` | `کاربر` | User — displayed as the **raw integer**, never resolved to a name |

**Button bar** (`sPanel1`), with permission key and target:

| Caption | English | Permission | Precondition | Action |
|---|---|---|---|---|
| `دریافت چک جدید` | Receive new cheque | 2102 | year open | `CheckDaryaftF.new` |
| `اصلاح چک` | Amend cheque | 2103 | state = 1, same year, voucher draft | `CheckDaryaftF.edit(S_SSN)` |
| `حذف چک` | Delete cheque | 2104 | state = 1, same year | **no-op** (§2.3 T3) |
| `واگذاری به بانک` | Hand over to the bank | 2105 | state ≤ 1, `S_COID ≤ CO_ID` | `CheckDaryaft2F.New(S_SSN)` |
| `برگشت از بانک` | Return from the bank (bounce) | 2106 | state = 2 | `CheckBargashtF.New(S_SSN)` |
| `وصول چک` | Collect cheque | 2107 | state = 2 | `CheckVosoolF.init(S_SSN)` |
| *(no caption)* `S_DVosool` | *(undo collection)* | 2108 | — | **no handler — dead button** |
| `استردادچک` | Return cheque to issuer | 2109 | state ≤ 1 | `CheckEsterdadF.init(S_SSN)` |
| `برگشت` | Back | — | — | `Close` |

**Toolbar strip** (`sPanel3`), left to right: `S_Search` (hint `جستجو` "search" — resets all filters
and reloads, the only working control), `State1`…`State5` (each `Tag` = its state code),
`B_SarResid` (due-date filter), `B_Names` (name search), `S_Print` (opens the print popup),
`GridFontSize` (a `TsUpDown` that scales both grids, 6-15pt). **Every one of `State1`-`State5`,
`B_SarResid` and `B_Names` lacks an `OnClick` handler** — see §5.5.

**Print popup**: `Print1` → `RP1` with the title chosen by `_State` / `_Date`
(`CheckListDU.pas:256-272`), which in practice is always `' لیست همه چکها'` "list of all cheques";
`Print2` and `_N1` have no handler.

**Rebuild notes**: this screen is the single most important treasury view. It needs real filters
(state, counterparty, due-date range, fiscal year), server-side pagination (today it loads the
entire `DCheck` table), a resolved user name, and the state-colour coding that was written and then
disabled.

---

### 11.2 `CheckDaryaftU` — Receive a cheque

**Title**: `درریافت چک` — "receipt of cheque", **misspelt** (`دریافت` written `درریافت`),
`CheckDaryaftU.dfm:5`.
**Modes**: `new` (blank), `edit(SSN)`, `New_From_PRg(prg, factor, bes, date)` (invoice-linked).

| Field | Label | Control | Behaviour |
|---|---|---|---|
| `S_Bes` | `: دهنده چک` "cheque giver" | code + `...` picker `B_Bes` + `S_BesName` + `S_BesN` | required, must be a leaf. Read-only when `S_LinkPrg > 0`. Selecting it auto-fills `S_Bed` as `Sandoogh_KM + '-' + <Tafsil-1>` (`:187`) — or, via the picker button, as the hard-coded `'108-1-' + <Tafsil-1>` (`:401`), **a different formula** |
| `S_Bed` | `: اسناد دریافتی` "notes receivable" | code + `...` picker `B_Bed` + names | required (`Tag > 0`), leaf-ness **not** checked. Right-click → save as default |
| `S_CheckNo` | `: شماره چک ` "cheque number" | `TsEdit` | **no validation** |
| `S_Sanad` | `: شماره سند` "voucher number" | `TEditInt` | **read-only** (`:384`); overwritten at save by `Get_NewSanad_DateID` |
| `S_Date` | `: تاریخ سند` "voucher date" | `TFullDate` | required, must be inside the fiscal year. Defaults to today |
| `S_Dates` | `: سررسید` "due date" | `TFullDate` | **no validation**. Defaults to tomorrow |
| `S_Mab` | `: مبلغ چک` "cheque amount" | `TEditInt` | required `> 0` |
| `S_Desc` | `: شرح سند` "voucher description" | `TsEdit` | required non-blank |

**Buttons**: `ذخیره` "Save" (`B_Save`) runs the eight validations of §9.1 then the save of §10.2 and
closes; `برگشت` "Back" (`sBitBtn4`) closes without saving.
**Tab order** starts at `S_Bes` (`ActiveControl := S_Bes`, `:386`).

**Rebuild notes**: `Delete_Check` (`:411-441`) must be wired to a real delete button; the two
different default-account formulas must be reconciled; `S_CheckNo` and `S_Dates` need validation.

---


---

[← 10. SQL and stored procedures (part c)](06-10-c-sql-and-stored-procedures.md) | [index](00-index.md) | [11. Screen specifications (part b) →](06-11-b-screen-specifications.md)
