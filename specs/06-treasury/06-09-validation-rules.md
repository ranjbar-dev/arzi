_Part of [06-treasury](../06-treasury.md) — [index](00-index.md)_

## 9. Validation rules

Every rule is a client-side `MessageDlg(...)` followed by `exit`. There are **no** database
constraints, triggers or check constraints visible from this repository, and **no** server-side
re-validation: the same `TADOQuery` that shows the dialog also builds the SQL.

### 9.1 Received cheque — `CheckDaryaftU` (create/edit)

| # | Persian message (verbatim) | English | Condition | `file:line` |
|---|---|---|---|---|
| 1 | `کد بستانکار را انتخاب کنید` | "Select the credit-side (payer) account code" | `S_Bes.Tag <= 0` — the typed code did not resolve to an account | `CheckDaryaftU.pas:205` |
| 2 | `‌ کد بستانکار اشتباه است ` | "The credit-side account code is wrong" | `not Dm.is_Sarfasl_Last_Deep_SSN(S_Bes.Tag)` — the account is not a leaf | `CheckDaryaftU.pas:210` |
| 3 | `  کد اسناد دریافتی نزد صندوق را ایجاد کنید   ` | "Create the 'notes receivable in the cash box' account code" | `S_Bed.Tag <= 0` | `CheckDaryaftU.pas:223` |
| 4 | `‌ جهت ذخیره اطلاعات سند  <N>  را در حالت تحریر قرار دهید` | "To save, put voucher N back into draft state" | `Dm.Get_SanadMaxTX(S_Sanad) > 0` | `CheckDaryaftU.pas:230` |
| 5 | `‌ مبلغ چک را وارد کنید ` | "Enter the cheque amount" | `S_Mab.IntValue <= 0` | `CheckDaryaftU.pas:238` |
| 6 | `  تاریخ سند را وارد کنید  ` | "Enter the voucher date" | `S_Date.Farsi_Valid = false` | `CheckDaryaftU.pas:245` |
| 7 | `  تاریخ باید در رنج  <From_Date> الی <To_Date> باشد ` | "The date must be in the range F to T" | date outside the fiscal year | `CheckDaryaftU.pas:252-254` |
| 8 | `  شرح سند را وارد کنید ` | "Enter the voucher description" | `Trim(S_Desc.Text) = ''` | `CheckDaryaftU.pas:261` |
| 9 | `  چک مورد نظر پیدا نشد  ` | "The requested cheque was not found" | `Locate('S_SSN', …)` failed when opening in edit mode | `CheckDaryaftU.pas:115` |
| 10 | `‌ چک مورد نظر پیدا نشد ` | "The requested cheque was not found" | same, in `Delete_Check` | `CheckDaryaftU.pas:419` |
| 11 | `‌ سند معین را در حالت تحریر قار دهید ` (note the typo `قار` for `قرار`) | "Put the subsidiary voucher into draft state" | `Dm.Moein_Tx(_Sanad) > 0` in `Delete_Check` | `CheckDaryaftU.pas:425` |
| 12 | `‌ حذف چک ` / `‌ حذف چک <no> ` / `‌ سررسید <due> ` / `مطمئن هستید ؟` | "Delete cheque" / "Delete cheque N" / "Due D" / "Are you sure?" — a 4-part `GetYes` confirmation | before deleting | `CheckDaryaftU.pas:430-431` |
| 13 | `  ذخیره انجام شد ` | "Save completed" (info) | on success | `CheckDaryaftU.pas:363` |

**Not validated on a received cheque** — every one of these is a real gap:

- `S_CheckNo` may be blank, may be any string, and **is never checked for duplication**. Two
  identical cheque numbers from the same drawer are accepted silently.
- `S_DateS` (due date) is **never validated at all** — not for format, not for being a valid Jalali
  date, not for being ≥ the receipt date, not for being inside the fiscal year. It defaults to
  *tomorrow* (`CheckDaryaftU.pas:383`) and the operator may overwrite it with anything the
  `TFullDate` control accepts.
- `S_Bed` (the notes-receivable account) is checked for `> 0` but **not** for leaf-ness, unlike
  `S_Bes`.
- No maximum amount, no per-user limit, no duplicate-amount warning.

### 9.2 Cheque list — `CheckListDU` transition guards

| # | Persian message | English | Condition | `file:line` |
|---|---|---|---|---|
| 14 | `‌ جهت واگذاری چک به بانک باید چک در صندوق موجود باشد ` | "To hand the cheque to the bank, the cheque must be in the cash box" | `S_State > 1` on the deposit button | `CheckListDU.pas:351` |
| 15 | `‌ جهت واگذاری چک به بانک به سال قبل از دریافت چک مجاز نمی باشدی ` (trailing `ی` is a typo) | "Depositing the cheque to the bank in a year before its receipt is not permitted" | `S_COID > Dm.CO_ID` | `CheckListDU.pas:358`, repeated verbatim at `:422` |
| 16 | `‌ جهت استرداد چک  باید چک در صندوق موجود باشد ` | "To return the cheque, it must be in the cash box" | `S_State > 1` on the return button | `CheckListDU.pas:383` |
| 17 | `‌ جهت استرداد چک به شخص به سال قبل از دریافت چک مجاز نمی باشد ` | "Returning the cheque to a person in a year before its receipt is not permitted" | `S_COID > Dm.CO_ID` | `CheckListDU.pas:390` |
| 18 | `‌  جهت برگشت چک از بانک ، چک باید در بانک باشد .  ` | "To bounce the cheque from the bank, the cheque must be at the bank" | `S_State <> 2` | `CheckListDU.pas:415` |
| 19 | `‌ جهت اصلاح یا حذف چک باید در سال مالی صدور چک قرار داشته باشید ` | "To amend or delete a cheque you must be in the fiscal year the cheque was issued in" | `S_COID <> Dm.CO_ID` | `CheckListDU.pas:447` and `:508` |
| 20 | `‌ چک در صندوق نیست ` | "The cheque is not in the cash box" | `S_State <> 1` on edit/delete | `CheckListDU.pas:453` and `:501` |
| 21 | `‌ جهت اصلاح یا حذف چک سند معین را در حالت تحریر قرار دهید ` | "To amend or delete a cheque, put the subsidiary voucher into draft state" | `Dm.Moein_Tx(S_Sanad) <> 0` | `CheckListDU.pas:465` **(unreachable — see below)** and `:515` |
| 22 | `ایا برای حذف چک مطمئن هستید؟` (`آیا` misspelt `ایا`) | "Are you sure you want to delete the cheque?" | confirmation | `CheckListDU.pas:469` **(unreachable)** |
| 23 | `‌ جهت اعلام وصول ، چک باید از قبل به بانک واگذار شده باشد  ` | "To declare collection, the cheque must already have been handed to the bank" | `S_State <> 2` | `CheckListDU.pas:562` |
| 24 | `‌ جهت وصول چک در سال مالی قبل از واگذاری چک به بانک مجاز نمیباشدی  ` | "Collecting a cheque in a fiscal year before it was handed to the bank is not permitted" | `S_COID > Dm.CO_ID` | `CheckListDU.pas:569` |

Note the asymmetry: **edit and delete require `S_COID = Dm.CO_ID` exactly**, while deposit, bounce,
return and collect require only `S_COID <= Dm.CO_ID`. So a cheque received in year N can be worked
in year N+1 but never corrected there.

### 9.3 The delete protection

Three independent layers, only two of which work.

**(a) `TDM.QCheckBeforeDelete` (`Dmu.pas:1273-1276`) — the global dataset guard:**

```pascal
procedure TDM.QCheckBeforeDelete(DataSet: TDataSet);
begin
    Abort;
end;
```

An unconditional `Abort`, wired as the `BeforeDelete` handler of **five** shared datasets in
`Dmu.dfm` — `TCheck` (`:888`, via the `Dmu.dfm:29` assignment on the module), `QCheck` (`:898`),
`QDCheck` (`:910`), `DCheck` (`:922`) and `QDFish` (`:1008`). Effect: **no cheque or deposit-slip row
can ever be deleted through a dataset's `.Delete` method.** `Abort` raises `EAbort`, which the VCL
swallows silently — the user sees nothing at all, not even an error. Deletion is only possible
through the hand-written `Delete From …` SQL in the individual screens, which bypasses the dataset
layer entirely.

This is the reason `Dm.DCheck.Append` / `.Edit` / `.Post` are used for inserts and updates
(`CheckDaryaftU.pas:277-309`) while every delete is raw SQL.

**(b) Form-local guards:** `CheckListDU.Q1BeforeDelete` and `Q2BeforeDelete` (`:274-277, 295-298`)
also `Abort`; `CheckEditU.CD1BeforeDelete` (`:531-534`) and `TankhahEdit.CD1BeforeDelete`
(`:509-512`) have their `Abort` **commented out**, which is deliberate — those are in-memory
`TVirtualTable` grids where line deletion must work.

**(c) The `S_Delete` handler itself is a no-op** because of the unconditional `Exit;` at
`CheckListDU.pas:457` (§2.3 T3). Rules 21 and 22 above therefore never fire from that screen.

### 9.4 Deposit slip — `FISHDaryaftU` / `FishListD`

| # | Persian message | English | Condition | `file:line` |
|---|---|---|---|---|
| 25 | `کد بستانکار را انتخاب کنید` | "Select the credit-side (payer) account code" | `S_Bes.Tag <= 0` | `FISHDaryaftU.pas:363` |
| 26 | `کد حساب بانک  را وارد کنید` | "Enter the bank account code" | `S_Bed.Tag <= 0` | `FISHDaryaftU.pas:369` |
| 27 | `سند در حالت تحریر نیست` | "The voucher is not in draft state" | `_OldSanad > 0 and Dm.Get_SanadMaxTX(_OldSanad) > 0` | `FISHDaryaftU.pas:381` |
| 28 | `‌ مبلغ فیش را وارد کنید ` | "Enter the slip amount" | `S_Mab.IntValue <= 0` | `FISHDaryaftU.pas:389` |
| 29 | `  تاریخ سند را وارد کنید  ` | "Enter the voucher date" | invalid Jalali date | `FISHDaryaftU.pas:396` |
| 30 | `  تاریخ باید در رنج  <F> الی <T> باشد ` | "The date must be in the range F to T" | outside fiscal year | `FISHDaryaftU.pas:403-404` |
| 31 | `‌ فیش مورد نظر پیدا نشد ` | "The requested slip was not found" | `Locate` failed in `Delete_Fish` | `FISHDaryaftU.pas:166` |
| 32 | `‌ سند معین را در حالت تحریر قار دهید ` | "Put the subsidiary voucher into draft state" (same typo) | `Dm.Moein_Tx > 0` in `Delete_Fish` | `FISHDaryaftU.pas:172` |
| 33 | `‌ حذف فیش ` / `‌ حذف فیش <no> ` / `‌ تاریخ <date> ` / `مطمئن هستید ؟` | 4-part `GetYes` delete confirmation | | `FISHDaryaftU.pas:177-178` |
| 34 | `    اطلاعات ذخیره شد     ` | "The data was saved" (info) | on success | `FISHDaryaftU.pas:491` |
| 35 | `‌ جهت اصلاح یا حذف واریزی باید در سال مالی صدور چک قرار داشته باشید ` (says "cheque" in a deposit-slip message) | "To amend or delete a deposit you must be in the fiscal year the **cheque** was issued in" | wrong fiscal year | `FishListD.pas:261` |
| 36 | `‌ جهت اصلاح یا حذف واریزی سند معین را در حالت تحریر قرار دهید ` | "To amend or delete a deposit, put the subsidiary voucher into draft state" | voucher not in draft | `FishListD.pas:267` and `:315` |
| 37 | `‌ از برنامه جانبی برای حذف استفاده کنید ` | "Use the side/auxiliary program to delete" | when the slip has `S_LinkPRG > 0`, i.e. it was created by another module | `FishListD.pas:273` |
| 38 | `ایا برای حذف واریزی مطمئن هستید؟` | "Are you sure you want to delete the deposit?" | confirmation | `FishListD.pas:277` |
| 39 | `‌ جهت اصلاح یاحذف واریزی باید در سال جاری باشید ` (missing space in `یاحذف`) | "To amend or delete a deposit you must be in the current year" | wrong fiscal year, edit path | `FishListD.pas:309` |

**Not validated on a deposit slip**: `S_FishNo` may be blank and is never checked for duplication;
`S_Desc` is **not** required (unlike the cheque screen); neither account is checked for leaf-ness
(unlike the cheque screen); there is no channel-specific requirement (a `کارت به کارت` transfer
needs no reference number).

### 9.5 Issued-cheque batch — `CheckEditU` / `CheckEditAddU` / `CheckListU`

| # | Persian message | English | Condition | `file:line` |
|---|---|---|---|---|
| 40 | `سند در حالت تحریر نیست` | "The voucher is not in draft state" | `max(M_Tx) > 0` for `M_Id=26, M_Link=_SSN` | `CheckEditU.pas:365` |
| 41 | `  کد بانک را وارد کنید  ` | "Enter the bank code" | `CM_Code.Tag = 0` | `CheckEditU.pas:373` |
| 42 | `  مبلغ  را وارد کنید  ` | "Enter the amount" | `CM_Mab.IntValue = 0` (the computed footer sum) | `CheckEditU.pas:380` |
| 43 | `  تاریخ را وارد کنید  ` | "Enter the date" | invalid Jalali date | `CheckEditU.pas:387` |
| 44 | `  تاریخ در رنج مجاز نیست  ` | "The date is not in the permitted range" | outside fiscal year | `CheckEditU.pas:394` |
| 45 | `  شرح لیست را وارد کنید  ` | "Enter the list description" | `Trim(CM_Desc) = ''` | `CheckEditU.pas:401` |
| 46 | `  پیدا نشد  ` | "Not found" | `LoadSSN` found no `CheckMaster` row | `CheckEditU.pas:142` |
| 47 | `  لیست ذخیره شد   ` | "The list was saved" (info) | success | `CheckEditU.pas:518` |
| 48 | `کد بدهکار حساب را وارد کنید` | "Enter the debit account code" | payee line: account did not resolve | `CheckEditAddU.pas:62` |
| 49 | `‌ مبلغ را وارد کنید` | "Enter the amount" | payee line amount is zero | `CheckEditAddU.pas:68` |
| 50 | `‌ شرح را وارد کنید` | "Enter the description" | payee line description blank | `CheckEditAddU.pas:74` |
| 51 | `‌ سند <N> را در حالت تحریر قرار دهید .` | "Put voucher N into draft state." | `Max(DM_TX) > 0` on the header, before edit or delete | `CheckListU.pas:174` and `:224` |
| 52 | `     برای حذف لیست مطمئن هستید؟      ` | "Are you sure you want to delete the list?" | confirmation (`mbYes/mbNo/mbCancel`) | `CheckListU.pas:180` |
| 53 | `   لیست حذف شد   ` | "The list was deleted" (info) | success | `CheckListU.pas:195` |

Note rule 45 has a comment `// check sum list` at `CheckEditU.pas:406` with **no code under it** —
the intended "the lines must sum to the header" check was never written. It cannot fail today
because `CM_Mab` *is* the footer sum, but it means the batch has no minimum line count: a batch with
zero payee lines has `CM_Mab = 0` and is rejected only by rule 42.

### 9.6 Petty cash — `TankhahEdit` / `TankhahEditAddu` / `TankhahList`

Identical in shape to §9.5 with `M_Id = 41`:

| # | Persian message | English | Condition | `file:line` |
|---|---|---|---|---|
| 54 | `سند در حالت تحریر نیست` | "The voucher is not in draft state" | `max(M_Tx) > 0` for `M_Id=41, M_Link=_SSN` | `TankhahEdit.pas:344` |
| 55 | `  کد تنخواه دار  را وارد کنید  ` | "Enter the petty-cash holder's code" | `TM_Code.Tag = 0` | `TankhahEdit.pas:352` |
| 56 | `  مبلغ  را وارد کنید  ` | "Enter the amount" | `TM_Mab.IntValue = 0` | `TankhahEdit.pas:359` |
| 57 | `  تاریخ را وارد کنید  ` | "Enter the date" | invalid Jalali date | `TankhahEdit.pas:366` |
| 58 | `  تاریخ در رنج مجاز نیست  ` | "The date is not in the permitted range" | outside fiscal year | `TankhahEdit.pas:373` |
| 59 | `  شرح لیست را وارد کنید  ` | "Enter the list description" | `Trim(TM_Desc) = ''` | `TankhahEdit.pas:380` |
| 60 | `  پیدا نشد  ` | "Not found" | `LoadSSN` found no row | `TankhahEdit.pas:136` |
| 61 | `  لیست ذخیره شد   ` | "The list was saved" (info) | success | `TankhahEdit.pas:496` |
| 62 | `کد بدهکار حساب را وارد کنید` | "Enter the debit account code" | expense line account unresolved | `TankhahEditAddu.pas:56` |
| 63 | `‌ مبلغ را وارد کنید` | "Enter the amount" | expense line amount zero | `TankhahEditAddu.pas:62` |
| 64 | `‌ شرح را وارد کنید` | "Enter the description" | expense line description blank | `TankhahEditAddu.pas:68` |
| 65 | `‌ سند <N> را در حالت تحریر قرار دهید .` | "Put voucher N into draft state." | header voucher not in draft | `TankhahList.pas:139` and `:196` |
| 66 | `   آیا برای حذف لیست تنخواه مطمئن هستید ؟     ` + newline + `      لیست قابل برگشت نمی باشد    ` | "Are you sure you want to delete the petty-cash list?" / "The list cannot be recovered" | confirmation | `TankhahList.pas:145` |
| 67 | `    لیست با موفقیت حذف شد    ` | "The list was deleted successfully" (info) | success | `TankhahList.pas:164` |

Note the confirmation at rule 66 tests `<> 6` rather than `<> mrYes` (`TankhahList.pas:145`) — a
magic number equal to `mrYes`, but brittle.

### 9.7 Cross-cutting rules (not treasury-specific but always in the path)

| # | Persian message | English | Source |
|---|---|---|---|
| 68 | `   سال مالی پیدا نشد  ` | "The fiscal year was not found" | `Dm.Is_New_Sanad_Valid`, `Dmu.pas:1005` |
| 69 | `          سال مالی مورد نظر بایگانی شده است                ` + newline + `   اجازه تغییر در این سال و صدور فاکتور و سند را ندارید    ` | "The selected fiscal year has been archived" / "You are not permitted to change anything, or issue invoices or vouchers, in this year" | `Dm.Is_New_Sanad_Valid`, `Dmu.pas:1010-1011` |
| 70 | *(silent)* | Every treasury action is additionally gated by `Dm.IsEnabel(Dm.userId, <key>)`, which merely disables the button — no message. Keys: 2102-2109 (received-cheque list, `CheckListDU.pas:239-246`), 2111-2114 + 2125 + 1121 (issued-cheque list, `CheckListU.pas:118-124`), 2114 (treasury printing). | `Dmu.pas:1552` |

### 9.8 Systemic observations

- **Every SQL statement in the treasury module is built by string concatenation.** Parameters are
  used only for the two batch screens' `INSERT`s (`CheckEditU.pas:427-438`,
  `TankhahEdit.pas:406-416`). Everything else interpolates `QuotedStr(...)` at best and raw
  `inttostr(...)` at worst. `QuotedStr` doubles single quotes, so Persian free text is safe from
  breaking the statement, but there is no defence in depth.
- **The same validation is written out longhand in six places** (the date-range check appears
  verbatim in `CheckDaryaftU`, `CheckDaryaft2U`, `CheckBargashtu`, `CheckEsterdadU`,
  `FISHDaryaftU`, `CheckVosoolU`) with three different message wordings.
- **No uniqueness rule exists anywhere in treasury** — not for cheque numbers, slip numbers, batch
  numbers or claim numbers.
- **No validation is repeated on the server.** Any client that talks to the database directly can
  write any state.


---

[← 8. Accounting integration](06-08-accounting-integration.md) | [index](00-index.md) | [10. SQL and stored procedures (part a) →](06-10-a-sql-and-stored-procedures.md)
