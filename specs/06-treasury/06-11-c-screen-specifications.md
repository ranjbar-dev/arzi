_Part of [06-treasury](../06-treasury.md) — [index](00-index.md)_

### 11.9 `CheckListU` — Issued-cheque batch list

**Title**: `   تنظیمات بانک` "bank settings" — **wrong** again (`CheckListU.dfm:5`).
**Entry**: `Mainu.pas:611` → `CheckListF.init`.
**Query**: `Select * From CheckMaster Where CM_Coid=:Coid` (year-scoped, no `ORDER BY`).

**Grid columns** (`DisplayLabel`s): `شماره چک` cheque/batch number (`CM_No`), `تاریخ` date
(`CM_Date`), `مبلغ` amount (`CM_Mab`), `شرح` description (`CM_Desc`), `کد بانک` bank code
(`CM_CodeCR`), `نام بانک` bank name (`CM_CodeName`), `تعداد نفرات` "number of persons"
(`CM_Count`), `سند` voucher (`CM_Sanad`).

**Buttons**:

| Caption | English | Permission | Guard | Action |
|---|---|---|---|---|
| `صدور چک جدید` | Issue a new cheque | 2111 | year open | `CheckEditF.new` |
| `اصلاح چک` | Amend cheque | 2112 | year open, `Max(DM_TX) = 0` | `CheckEditF.Edit(CM_SSN)` |
| `نمایش لیست` | Show list | 2113 | — | `CheckEditF.View(CM_SSN)` (read-only) |
| `حذف چک` | Delete cheque | 2125 | year open, `Max(DM_TX) = 0`, confirmation | full delete (§10.10) |
| `چاپ لیست حسابداری` | Print the accounting list | 2114 | — | loads into the editor, fires `RP1` |
| `چاپ لیست بانک` | Print the bank list | 2114 | — | loads into the editor, fires `RP2` |
| `مشاهده سند` | View the voucher | 1121 | `CM_Sanad > 0` | `SanadEditF.View(CM_Sanad)` |
| `برگشت` | Back | — | — | `Close` |

A print popup offers `لیست حسابداری` "accounting list", `چاپ لیست` "print list" and `لیست بانک`
"bank list"; `Print_3Click` renders `RP3` titled `لیست چکهای صادره` "list of issued cheques".

---

### 11.10 `CheckEditU` — Issued-cheque batch editor

**Title**: `صدور چک` "cheque issuance".

**Header fields** (top panel `P1`, disabled wholesale in View mode):

| Field | Label | Control |
|---|---|---|
| `CM_Code` | `کد حساب بانک` "bank account code" | code + `SB1` `...` picker + `CM_Name` (full path). Right-click `ذخیره کد بانک به عنوان پیش فرض` "save the bank code as default" |
| `CM_Date` | `تاریخ چک` "cheque date" | `TFullDate` |
| `CM_No` | `شماره چک` "cheque number" | `TsEdit` |
| `CM_Sanad` | `شماره سند` "voucher number" | `TEditInt`, auto |
| `CM_Tittle` | `عنوان لیست` "list heading" | `TsMemo`, 3 lines. Right-click `ذخیره توضیحات به عنوان پیش فرض` "save the notes as default" |
| `CM_Mab` | `مبلغ چک` "cheque amount" | `TEditInt`, **computed** from the grid footer, not typed |
| `CM_Desc` | `توضیح لیست` "list description" | `TsEdit` |

**Line grid `G1`** (in-memory `TVirtualTable CD1`, summary footer on the amount column):

| # | Field | Persian header | English header |
|---|---|---|---|
| 1 | `CD_Code` | `کد حساب` | Account code |
| 2 | `CD_Name` | `نام کد` | Account name |
| 3 | `CD_Jari` | `نام صاحب حساب` | Bank-account holder's name |
| 4 | `CD_Mab` | `مبلغ` | Amount (`DisplayFormat '###,###'`) |
| 5 | `CD_BankNo` | `شماره حساب / شماره شبا` | Account number / IBAN |
| 6 | `CD_Desc` | `شرح` | Description |

**Buttons**: `افزودن` "Add" → `CheckEditAddF.New`; `اصلاح` "Amend" → loads the current line into the
dialog; `حذف` "Delete" → removes the in-memory line; `ذخیره` "Save"; `لیست حسابداری` "accounting
list" → `RP1`; `لیست بانک` "bank list" → `RP2`; `خروج` "Exit".

After a successful save the whole panel is disabled and the mode flips to View
(`CheckEditU.pas:520-527`) — the user must reopen the record to make a further change.

**Reports**: `RP1` fills `T1` = company name, `T2` = `CM_Desc`, `T5` = `'تاریخ : '+date` + newline +
`'ســند : '+voucher`, `T4` = `'جمع : '+<amount in Persian words>+' ریال'` "total: … rials"
(`CheckEditU.pas:329-338`). `RP2` additionally fills `T3` = `CM_Tittle`. `RP1_Old` is declared and
never used.

---

### 11.11 `CheckEditAddU` — Payee-line dialog

**Title**: `انتخاب بدهکار و مبلغ` "select the debtor and the amount".

| Field | Label | English |
|---|---|---|
| `CD_Code` (+ `...` picker, + `CD_Name`) | `کد حساب بدهکار` | Debit account code |
| `CD_Mab` | `مبلغ پرداخت` | Payment amount |
| `CD_Desc` | `توضیح پرداخت` | Payment description |
| `CD_BankNo` | `شماره حساب` | Account number |
| `CD_Jari` | `گیرنده وجه` | Recipient of the funds |

Three validations (§9.5 rules 48-50). Buttons `خروج` "Exit" and `تایید` "Confirm"; confirm sets
`Tag := 1`.

---

### 11.12 `TankhahList` — Petty-cash claim list

**Title**: `لیست تنخواه ` "petty-cash list".
**Query**: `Select * From TankhahMaster Where TM_Coid=:Coid Order By TM_Date`; opens on the last row.

**Grid columns**: `شماره لیست` list number (`TM_No`), `شماره سند` voucher (`TM_Sanad`), `تاریخ` date
(`TM_Date`), `جمع لیست` list total (`TM_Mab`), `تعداد ایتم` item count (`TM_Count`), `شرح لیست`
list description (`TM_Desc`), `کد تنخواه` petty-cash code (`TM_CodeCR`), `نام تنخواه دار`
petty-cash holder's name (`TM_CodeName`).

**Buttons**: `لیست جدید` "New list" (2121), `اصلاح لیست` "Amend list" (2122),
`نمایش لیست` "Show list" (2123), `حذف لیست تنخواه` "Delete petty-cash list" (**no permission key**),
`چاپ لیست حسابداری` "Print accounting list" and `چاپ لیست تنخواه` "Print petty-cash list" (both 2124),
`مشاهده سند` "View voucher" (1121), `برگشت` "Back".

---

### 11.13 `TankhahEdit` — Petty-cash claim editor

**Title**: `لیست تنخواه` "petty-cash list". **Note**: this `.dfm` is stored in Delphi's **binary**
format, unlike every other form in the module — read it with a converter, not a text editor.

**Header fields**: `کد تنخواه دار` petty-cash holder's code (`TM_Code` + `SB1` picker + `TM_Name`
short name + `TM_FullName` full path), `تاریخ لیست` list date (`TM_Date`), `شماره لیست` list number
(`TM_No`), `شماره سند` voucher number (`TM_Sanad`), `عنوان لیست` list heading, `جمع لیست` list total
(`TM_Mab`, computed).

**Line grid** columns: `کد حساب` account code (`TD_Code`), `نام طرف حساب` counterparty name
(`TD_Name`), amount and description.

**Buttons**: Add / Amend / Delete line, `ذخیره` Save, `لیست حسابداری` accounting list, `چاپ لیست`
print list, `خروج` Exit. Two right-click popups mirror `CheckEditU`'s
(`ذخیره توضیحات به عنوان پیش فرض`, `ذخیره کد بانک به عنوان پیش فرض` — the latter caption is wrong
here, it saves the *custodian* code).

**Reports** `RP1`/`RP2` fill `T1` company name, `T2` description, `T4` total in Persian words,
`T5` = date + voucher + `'سریـال : ' + <TM_SSN>` "serial" (`TankhahEdit.pas:304-326`). The report
layout carries the hard-coded footer `تنظیم کننده … مدیرمالی` "prepared by … finance manager" and,
in the stored preview data, the company name
`شرکت تعاونی تولید کنندگان پسته رفسنجان` — **the customer's identity is baked into the form
resource**, not read from `Base`.

---

### 11.14 `TankhahEditAddu` — Expense-line dialog

**Title**: `انتخاب بدهکار تنخواه  و مبلغ` "select the petty-cash debtor and the amount".
Fields: `کد حساب بدهکار` debit account code (`TD_Code` + picker + `TD_FullName`),
`مبلغ پرداخت` payment amount (`TD_Mab`), `توضیح پرداخت` payment description (`TD_Desc`).
Buttons `خروج` Exit / `تایید` Confirm. Three validations (§9.6 rules 62-64).

---

### 11.15 `BankTanzim` — Bank settings (**inert, do not port as-is**)

**Title**: `   تنظیمات بانک` "bank settings". Never opened from anywhere (§1.7). Its grid columns
document the intended `banks` table: bank account code / name (`S1Code`, `S1_Name`), notes-payable
account code / name (`S2Code`, `S2_Name`), `BN_SSN`, `BN_Name`, `BN_BankCode`, `BN_AsnadCode`,
`BN_Shaba`, `BN_Check`, `BN_Fish`, plus the raw hierarchy columns `S1_Ko/Mo/Ta1/Ta2` and
`S2_Ko/Mo/Ta1/Ta2`. Two labelled edit fields exist: `کد حساب بانک` "bank account code" and
`کد حساب اسناد پرداختنی` "notes-payable account code". Five buttons, of which only
`sBitBtn4Click` (Close) is wired; `sBitBtn3` carries the untranslated caption `A_Select`.

Treat this form as a **requirements sketch for the rebuild's bank master**, not as a screen to
reproduce.


---

[← 11. Screen specifications (part b)](06-11-b-screen-specifications.md) | [index](00-index.md) | [12. Open questions →](06-12-open-questions.md)
