_Part of [06-treasury](../06-treasury.md) — [index](00-index.md)_

## 1. Entity model

Table DDL does not exist in this repository. Every column below is inferred from persistent-field
declarations in `.dfm` files (which carry the ADO-reported type and size) and from `INSERT` /
`UPDATE` statements in the `.pas` files. Nullability is stated as *inferred* wherever it rests on
"the code always writes it" rather than on a constraint we can see.

The treasury domain owns **eight** physical tables:

| Legacy table | Rows are | Proposed table |
|---|---|---|
| `DCheck` | received cheques (the only entity with a lifecycle) | `received_cheques` |
| `DCheck2` | one row per received-cheque state transition | `received_cheque_events` |
| `DFish` | bank deposit slips / incoming electronic transfers | `deposit_slips` |
| `CheckMaster` | header of an **issued** cheque batch (a bank payment run) | `cheque_payment_batches` |
| `CheckDetail` | one payee line of an issued cheque batch | `cheque_payment_batch_lines` |
| `TankhahMaster` | header of a petty-cash expense claim | `petty_cash_claims` |
| `TankhahDetail` | one expense line of a petty-cash claim | `petty_cash_claim_lines` |
| `TCheck` | *(declared in `Dmu.dfm:888-892`, never read or written by any code)* | drop |

There is **no bank table, no bank-account table, no branch table and no cheque-book table.** See
§1.7 and §1.8.

### 1.1 `DCheck` — received cheque

Authoritative source: the `DCheckS_*` persistent fields on `TDM.DCheck` (`Dmu.dfm:919-1002`).
Ordered as declared.

| Legacy column | Proposed name | Type | Meaning | Null? |
|---|---|---|---|---|
| `S_SSN` | `id` | `TAutoIncField` → `int IDENTITY` | Primary key. `Dmu.dfm:954-957` marks it read-only. | no |
| `S_COID` | `fiscal_year_id` | `int` | Fiscal year the cheque was **received** in, not the year of any later event. Written once from `Dm.CO_ID` at receipt (`CheckDaryaftU.pas:282`) and never updated by a transition. Drives the "you are in the wrong year" guards (`CheckListDU.pas:355, 445, 506`). | no |
| `S_State` | `status` | `int` | Lifecycle code. See §2. | no |
| `S_StateName` | *drop* | `varchar(50)` | Denormalised Persian label of `S_State`, written by whichever screen last transitioned the row. Redundant and, for state 1, ambiguous (§2.1). Derive from `status` in the rebuild. | no |
| `S_CheckNo` | `cheque_number` | `varchar(15)` | The number printed on the cheque. **Free text, never validated, never checked for uniqueness** (§9). | inferred yes — no validation forces it |
| `S_Sanad` | `voucher_number` | `int` | The treasury voucher (`Moein.M_Sanad`) the *receipt* posting landed in. Reallocated on every edit by `Dm.Get_NewSanad_DateID` (`CheckDaryaftU.pas:269`). Later transitions post to their own voucher numbers, which are **not** stored on `DCheck` — only on the matching `DCheck2` row. | no |
| `S_Date` | `received_on_jalali` | `varchar(10)` | Jalali receipt/voucher date, `YYYY/MM/DD` string. Range-checked against `Dm.From_Date` / `Dm.To_Date` (`CheckDaryaftU.pas:248-256`). | no |
| `S_DateS` | `due_date_jalali` | `varchar(50)` | **The cheque's due date (سررسید).** Declared `Size = 50` even though it holds a 10-character Jalali date — the widest field mismatch in the schema. Defaulted to *tomorrow* (`S_Dates.SetToDate(Date()+1)`, `CheckDaryaftU.pas:383`) and never validated. Sole sort key of the cheque list (`Order By S_Dates`, `CheckListDU.pas:329`) and sole basis of the aging filter (§5). | no |
| `S_Mab` | `amount` | `TLargeintField` → `bigint` | Face value in rials. `> 0` enforced (`CheckDaryaftU.pas:235-240`). Used verbatim by every posting; no partials. | no |
| `S_Desc` | `description` | `varchar(200)` | Free-text narration. Non-blank enforced (`CheckDaryaftU.pas:258-264`). | no |
| `S_BesSSN` | `payer_account_id` | `int` → `Sarfasl.S_SSN` | The **counterparty who handed the cheque over** — credited at receipt, debited on return. Must be a leaf account (`Dm.is_Sarfasl_Last_Deep_SSN`, `CheckDaryaftU.pas:209-212`). | no |
| `S_BesCR` | `payer_account_code` | `varchar(50)` | Denormalised 4-segment code string of the same account. Also a search target (`CheckListDU.pas:326`). | no |
| `S_BesName` | `payer_name` | `varchar(100)` | Denormalised **last** segment name (`Taraf.Get_LastName`, `CheckDaryaftU.pas:183`), not the full path. Search target and voucher-narration source. | no |
| `S_BedSSN` | `notes_receivable_account_id` | `int` → `Sarfasl.S_SSN` | The "notes receivable on hand" account debited at receipt. Defaulted to `Sandoogh_KM + '-' + <payer's Tafsil-1>` (`CheckDaryaftU.pas:187`). Becomes the *credit* side when the cheque is deposited and the *credit* side again when it is returned. | no |
| `S_BedCR` | `notes_receivable_account_code` | `varchar(50)` | Denormalised code string. | no |
| `S_BedName` | `notes_receivable_name` | `varchar(50)` | Denormalised name. **`Size = 50` vs `S_BesName`'s 100** — asymmetric truncation. | no |
| `S_Zssn` | `endorsee_account_id` | `int` | **Dead.** Never read, never written (§2.3 T8, §4). | — |
| `S_ZCR` | `endorsee_account_code` | `varchar(50)` | Dead. | — |
| `S_ZName` | `endorsee_name` | `varchar(100)` | Dead. | — |
| `S_linkPrg` | `source_module` | `int` | Which module created the cheque. `0` = entered by hand in the treasury; `1` = created from a goods invoice, displayed as `' فاکتور ' + S_LinkSSN` "invoice N" (`CheckListDU.pas:289-292`). No other value is handled. When `> 0` the payer account becomes read-only (`CheckDaryaftU.pas:132-133`). | no (defaults 0) |
| `S_LinkSSN` | `source_id` | `int` | Primary key in the source module — the invoice id when `S_linkPrg = 1`. Joined by `Anbar_Tasfieh` (`Dmu.dfm:1068-1069`). | no (defaults 0) |
| `S_UserID` | `created_by` | `int` → `User.id` | Who created the row. Overwritten on every edit (`CheckDaryaftU.pas:283`), so it is really "last editor". There is **no** `created_at`, `updated_at` or `updated_by`. | no |

**Missing columns the rebuild will need**: issuing bank, branch, account number, drawer/signatory
name (all of which the operator can only smuggle into `S_Desc`), plus a real `deposited_at`,
`cleared_at`, `bounced_at`, `returned_at` — today those dates exist only on `DCheck2`.

### 1.2 `DCheck2` — received-cheque event log

Columns are known from the two `INSERT` column lists and the field declarations on the history grid
(`CheckListDU.pas:99-110`, `CheckDaryaft2U.pas:192`, `CheckVosoolU.pas:225`).

| Legacy column | Proposed name | Type | Meaning | Null? |
|---|---|---|---|---|
| `S_SSN` | `id` | `int IDENTITY` | Primary key; also the event ordering key (`Order By S_SSN`, `CheckListDU.pas:164`). | no |
| `S_Link` | `received_cheque_id` | `int` → `DCheck.S_SSN` | Owning cheque. Not declared as an FK anywhere we can see. | no |
| `S_COID` | `fiscal_year_id` | `int` | Fiscal year **of the event**, which may differ from `DCheck.S_COID`. | no |
| `S_Sanad` | `voucher_number` | `int` | Voucher this event posted to. | no |
| `S_Date` | `event_date_jalali` | `varchar(10)` | Jalali date the operator entered for the event (deposit / bounce / collection / return date). | no |
| `S_Mab` | `amount` | `bigint` | Always a copy of `DCheck.S_Mab`. | no |
| `S_State` | `status_after` | `int` | State the cheque is *supposed* to be in after this event. **Wrong for bounces** (§2.1). | no |
| `S_StateName` | *drop* | `varchar(50)` | Persian label; not the same string set as `DCheck.S_StateName` (`برگشت از بانک` vs `چک برگشت شده از بانک`, `استرداد چک` vs ` چک مسترد شد `). | no |
| `S_BedSSN` | `debit_account_id` | declared as `TStringField` on the grid (`CheckListDU.pas:107`) but always written an integer | Debit account of the event's posting. The string field type strongly suggests the physical column is `varchar`, i.e. **the type differs from `DCheck.S_BedSSN`** — verify against the live DB. | no |
| `S_BesSSN` | `credit_account_id` | `int` | Credit account of the event's posting. **Omitted by the collection screen** (`CheckVosoolU.pas:225`), so it is NULL/0 on every `S_State=5` row. | yes in practice |
| `S_Desc` | `description` | `varchar` | Event narration, prefixed by the screen (`'انتقال چک به بانک '`, `'برگشت چک از بانک '`, `'استرداد چک '`). The collection screen instead copies the cheque's original description. | no |
| `S_UserID` | `created_by` | `int` | Operator. No timestamp column. | no |

No `DCheck2` row is written for the initial receipt, and no code ever deletes or updates a
`DCheck2` row — including the (unused) `Delete_Check`, which orphans history.

### 1.3 `DFish` — bank deposit slip / incoming transfer

Field list from `FishListD.dfm:620-680` plus the `INSERT`/`UPDATE` column lists at
`FISHDaryaftU.pas:438-452`.

| Legacy column | Proposed name | Type | Meaning | Null? |
|---|---|---|---|---|
| `S_SSN` | `id` | `int IDENTITY` | Primary key. | no |
| `S_COID` | `fiscal_year_id` | `int` | Fiscal year. | no |
| `S_State` | `deposit_method` | `int` 1-4 | **Not a lifecycle** — a deposit *channel*, taken from the combo's `ItemIndex + 1` (`FISHDaryaftU.pas:418`). A deposit slip has no states at all. Values in §6. | no |
| `S_StateName` | *drop* | `varchar(50)` | Persian label of the channel, copied from the combo's `Text` (`FISHDaryaftU.pas:432`). | no |
| `S_FishNo` | `slip_number` | `varchar(15)` | The bank's slip/reference number. Free text, **never validated, never unique** (§9). | inferred yes |
| `S_Sanad` | `voucher_number` | `int` | Treasury voucher. Reallocated from the date on every save (`FISHDaryaftU.pas:375`). | no |
| `S_Date` | `deposit_date_jalali` | `varchar(10)` | Jalali date; range-checked. | no |
| `S_Mab` | `amount` | `bigint` | `> 0` enforced (`FISHDaryaftU.pas:386-391`). | no |
| `S_Desc` | `description` | `varchar(200)` | Narration. **Not** checked for blankness, unlike the cheque screen. | yes |
| `S_BesSSN` | `payer_account_id` | `int` | Counterparty who paid the money in — credited. `> 0` enforced (`FISHDaryaftU.pas:361-365`), but **leaf-ness is not** (contrast the cheque screens). | no |
| `S_BesCR` | `payer_account_code` | `varchar(25)` per the T-SQL local declared at `FISHDaryaftU.pas:427` | Denormalised code. | no |
| `S_BesName` | `payer_name` | `varchar(200)` per `FISHDaryaftU.pas:429` | Denormalised last-segment name. | no |
| `S_BankSSN` | `bank_account_id` | `int` | The bank/cash account credited with the money — **debited**. Note the screen calls this control `S_Bed` and the DB column `S_Bank*`; the mapping is `@BedSSN → S_BankSSN` (`FISHDaryaftU.pas:439-441`). `> 0` enforced. | no |
| `S_BankCR` | `bank_account_code` | `varchar(25)` | Denormalised code. | no |
| `S_BankName` | `bank_account_name` | `varchar(200)` | Denormalised name. | no |
| `S_UserID` | `created_by` | `int` | Operator; overwritten on edit. | no |
| `S_LinkPRG` | `source_module` | `int` | `0` manual, `1` created from a goods invoice (`FISHDaryaftU.pas:288`, and `Anbar_Tasfieh` joins on it, `Dmu.dfm:1065-1067`). | no |
| `S_LinkSSN` | `source_id` | `int` | Source-document id. | no |
| `S_DateS` | *(uncertain)* | ? | **Not declared** as a persistent field on any dataset, yet read at `FISHDaryaftU.pas:178` in the delete-confirmation dialog (`' تاریخ ' + dm.DFish.FieldByName('S_DateS')`). Either the physical column exists and is simply never written by this application, or that line raises at runtime. Cannot be resolved without the live schema — see §12. | — |

A `DFish` row has **no line items**: one slip = one amount = one counterparty = one bank account.
See §6 for what this means for the "grouping" question.

### 1.4 `CheckMaster` — issued-cheque batch header

Columns from the `INSERT` at `CheckEditU.pas:417-418` and the loader at `CheckEditU.pas:145-151`.

| Legacy column | Proposed name | Type | Meaning | Null? |
|---|---|---|---|---|
| `CM_SSN` | `id` | `int IDENTITY` (`CheckListU.dfm:520-522`) | Primary key; recovered with `SELECT @@IDENTITY` (`CheckEditU.pas:424`). | no |
| `CM_Coid` | `fiscal_year_id` | `int` | Fiscal year; the list filters on it (`Select * From CheckMaster Where CM_Coid=:Coid`, `CheckListU.dfm:517`). | no |
| `CM_No` | `batch_number` | `varchar` | Operator-supplied reference ("شماره چک" in the grid title). Free text, not validated. | yes |
| `CM_Sanad` | `voucher_number` | `int` | Treasury voucher, reallocated from the date on every save (`CheckEditU.pas:409`). | no |
| `CM_Date` | `issue_date_jalali` | `varchar(10)` | Jalali date, range-checked (`CheckEditU.pas:392-397`). | no |
| `CM_Mab` | `total_amount` | `bigint` | **Computed, not entered** — the grid footer sum of the detail lines (`Set_Sum`, `CheckEditU.pas:252-256`). Enforced `<> 0`. | no |
| `CM_Desc` | `description` | `varchar` | Narration; non-blank enforced (`CheckEditU.pas:399-404`). | no |
| `CM_Tittle` | `letter_body` | `varchar` (multi-line) | A three-line free-text block used as the covering-letter body on report `RP2`; its default is remembered per-user in the INI under `T11`/`T12`/`T13` (`CheckEditU.pas:180-185, 546-548`). Note the legacy misspelling of "Title". | yes |
| `CM_Code` | `bank_account_id` | `int` → `Sarfasl.S_SSN` | The bank account the whole batch is **credited** from. Validated only as `Tag <> 0`, i.e. that the typed code resolved (`CheckEditU.pas:371-376`); leaf-ness is **not** checked. | no |
| `CM_CodeCR` | `bank_account_code` | `varchar` | Denormalised code; default remembered in the INI (`CheckEditU.pas:177, 540`). | no |
| `CM_CodeName` | `bank_account_name` | `varchar` | Denormalised **full** path name (`Taraf.Get_FullName`, `CheckEditU.pas:234`). | no |
| `CM_Count` | `line_count` | `int` | Cached `CD1.RecordCount` (`CheckEditU.pas:437`) — a denormalised count with the same drift risk as `DMoein`'s totals. | no |
| `CM_UserID` | `created_by` | `int` | Operator; overwritten on edit. | no |

### 1.5 `CheckDetail` — issued-cheque batch line

Columns from `CheckEditU.pas:453-454`; the surrogate key is not referenced by any code, so its name
is unknown (`CD_SSN` in the in-memory `TVirtualTable` is *not* the row key — it holds the debit
**account** id, `CheckEditU.pas:280`).

| Legacy column | Proposed name | Type | Meaning | Null? |
|---|---|---|---|---|
| *(unknown)* | `id` | `int IDENTITY` | Presumed identity PK; never read. | no |
| `CD_CMSSN` | `batch_id` | `int` → `CheckMaster.CM_SSN` | Owning batch. Lines are deleted and re-inserted wholesale on every save (`CheckEditU.pas:447`), so ids are not stable. | no |
| `CD_Coid` | `fiscal_year_id` | `int` | Redundant copy of the header's year. | no |
| `CD_Bed` | `payee_account_id` | `int` → `Sarfasl.S_SSN` | The payee, debited. | no |
| `CD_BedCR` | `payee_account_code` | `varchar` | Denormalised code. | no |
| `CD_BedName` | `payee_name` | `varchar` | Denormalised name. | no |
| `CD_Mab` | `amount` | `bigint` | Line amount; sums into `CM_Mab`. | no |
| `CD_Desc` | `description` | `varchar` | Line narration — becomes the voucher line's `Article` verbatim (`CheckEditU.pas:494`). | yes |
| `CD_BankNo` | `payee_bank_account_number` | `varchar(26)` (`CheckEditU.dfm:485-487`) | **The payee's bank account number or IBAN.** Grid label `شماره حساب / شماره شبا` "account number / SHABA (IBAN) number"; the entry-dialog label is the shorter `شماره حساب` (`CheckEditAddU.dfm:75`). Size 26 = the length of an Iranian IBAN (`IR` + 24 digits). Free text, never validated — note `TUtil.IS_ShabaNo` exists (`Utility.pas:90`) and is **not** called here. | yes |
| `CD_Jari` | `payee_account_holder_name` | `varchar(200)` (`CheckEditU.dfm:470-473`) | **The name on the payee's bank account**, which may differ from the `Sarfasl` account name. Grid label `نام صاحب حساب` "account holder's name"; the entry-dialog label is `گیرنده وجه` "recipient of the funds" (`CheckEditAddU.dfm:86`). Carried through the batch to the covering letter; **never used in any posting**. The name `Jari` (جاری, "current [account]") is misleading. | yes |

### 1.6 `TankhahMaster` / `TankhahDetail` — petty-cash claim

Structurally a clone of `CheckMaster`/`CheckDetail` with the prefix changed and two columns dropped.
Columns from `TankhahEdit.pas:396-397` and `:432-433`.

`TankhahMaster`: `TM_SSN` (`id`), `TM_Coid` (`fiscal_year_id`), `TM_No` (`claim_number`, free text),
`TM_Sanad` (`voucher_number`), `TM_Date` (`claim_date_jalali`), `TM_Mab` (`total_amount`, computed
from the lines — `TankhahEdit.pas:232-236`), `TM_Desc` (`description`, non-blank enforced),
`TM_Code` (`custodian_account_id` — the تنخواه‌دار / petty-cash holder, credited),
`TM_CodeCR` (`custodian_account_code`), `TM_CodeName` (`custodian_name`, the **last** segment —
`Taraf.Get_LastName`, `TankhahEdit.pas:211`), `TM_Count` (`line_count`), `TM_UserID` (`created_by`).
There is **no `TM_Tittle`** — the covering-letter block exists only on the cheque batch.

`TankhahDetail`: unknown identity PK, `TD_TMSSN` (`claim_id`), `TD_Coid` (`fiscal_year_id`),
`TD_Bed` (`expense_account_id`, debited), `TD_BedCR` (`expense_account_code`), `TD_BedName`
(`expense_account_name` — populated from `TD_FullName`, i.e. the **full** path, `TankhahEdit.pas:262`,
inconsistently with the header's last-segment name), `TD_Mab` (`amount`), `TD_Desc` (`description`,
becomes the voucher `Article`). There is **no `TD_BankNo` / `TD_Jari`** — the two banking-reference
columns of `CheckDetail` are absent.

### 1.7 Bank, bank account and branch — **not modelled**

- A "bank account" in this system is simply a leaf node of `Sarfasl` (the chart of accounts). Every
  screen that needs one uses the `Taraf` 4-segment account picker.
- `BankTanzim` (`BankTanzim.pas`, `BankTanzim.dfm`) is the only screen that looks like a bank
  master. Its grid declares columns over a table `BN_*` — `BN_SSN`, `BN_Name`, `BN_BankCode`,
  `BN_AsnadCode`, `BN_Shaba` (IBAN/SHABA), `BN_Check`, `BN_Fish` — plus two joined `Sarfasl`
  code/name pairs `S1_*` (labelled `کد حساب بانک` "bank account code") and `S2_*` (labelled
  `کد حساب اسناد پرداختنی` "notes-payable account code") (`BankTanzim.dfm:44-140, 161-211`).
  **The form is inert**: `DS1` has no `DataSet` assigned (`BankTanzim.dfm:304`), there is no query
  component on the form at all, `Q1CalcFields` (`BankTanzim.pas:69-87`) is an orphaned handler with
  no owner, and `BankTanzimF` is never instantiated or shown — `Mainu.pas:286` lists the unit in its
  `uses` clause and nothing else in the repository references the form variable.
  So either the `BN_*` table does not exist, or it exists and is unused. Treat the column list as a
  *design intent* for the rebuild's `banks` table, not as live data. See §12.
- Consequently, the bank a cheque is drawn on, the branch, and the drawer's account number are
  **not recorded anywhere** for received cheques.

### 1.8 Cheque book — **does not exist**

There is no cheque-book, cheque-stock, series or serial-range entity anywhere in the codebase. For
issued cheques the only number is the free-text `CM_No` on the batch header; individual issued
cheques do not carry numbers at all (a batch line has an amount and a payee bank account, not a
cheque number). Grep for `Daste`, `Book`, `Serial` in the treasury units returns nothing relevant.

### 1.9 Petty-cash fund — **also not an entity**

There is no `fund` table, no float/imprest amount, no assigned custodian record and no
replenishment document. A "petty-cash fund" is nothing more than a leaf `Sarfasl` account that the
operator happens to select as `TM_Code`. The running balance is whatever the general ledger says
about that account. See §7.

### 1.10 Cross-entity relationship summary

```
Sarfasl (leaf node) ──┬── DCheck.S_BesSSN   (payer)
                      ├── DCheck.S_BedSSN   (notes receivable on hand)
                      ├── DFish.S_BesSSN / S_BankSSN
                      ├── CheckMaster.CM_Code (bank)  ── CheckDetail.CD_Bed (payee)
                      └── TankhahMaster.TM_Code (custodian) ── TankhahDetail.TD_Bed (expense)

DCheck  1 ──* DCheck2                 (S_Link)
DCheck  * ──1 <source document>       (S_linkPrg + S_LinkSSN, only value 1 = goods invoice)
DFish   * ──1 <source document>       (S_LinkPRG + S_LinkSSN)
Moein   * ──1 <treasury document>     (M_Id + M_Link — see §8)
```


---

_start_ | [index](00-index.md) | [2. The cheque state machine →](06-02-cheque-state-machine.md)
