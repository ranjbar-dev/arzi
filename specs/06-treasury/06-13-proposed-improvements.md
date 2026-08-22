_Part of [06-treasury](../06-treasury.md) — [index](00-index.md)_

## 13. PROPOSED IMPROVEMENTS (needs user approval)

> **Nothing in this section is a port requirement.** The default decision is **port the behaviour
> as-is**, defects included. Everything below is a suggestion that changes observable behaviour and
> therefore needs explicit sign-off before it is built. They are grouped by how strongly the evidence
> in §§1-11 argues for them.

### 13.A Fix-on-port candidates — the legacy behaviour is unambiguously wrong

| # | Proposal | What it fixes | Risk of *not* doing it |
|---|---|---|---|
| A1 | **Split state 1 into `InHand` and `Bounced`.** Today "never deposited" and "deposited then bounced" share code 1 and differ only in a free-text label (§2.1). | Makes the state machine legible and lets the UI show bounced cheques distinctly. | Bounced cheques are invisible as a category. There is no way to report on them. |
| A2 | **Make `DCheck2` record the state the cheque was actually put into.** The bounce writes `2` while setting the master to `1` (§2.1, §10.4). | The event log stops contradicting the master row. | Any audit built on `DCheck2` is wrong for every bounce. |
| A3 | **Build the voucher header on collection.** `CheckVosoolU` is the only screen with no `DMoein_Make`/`Dmoein_UpdateMab` (§8.5 defect 1). | Collections stop producing headerless vouchers and stale `DM_Mab`. | Trial balances silently disagree with the ledger after any collection. |
| A4 | **Swap the two `DFish` narration strings back onto their correct sides** (§8.5 defect 2). | The daybook reads correctly. | Every deposit's journal narration says the opposite of what happened. |
| A5 | **Scope the hierarchy-repair `UPDATE` to the document, not the voucher** (§8.4, §10.7). | Saving one document stops rewriting columns on unrelated documents. | Currently harmless, but it is a cross-document write with no guard. |
| A6 | **Fix or remove the dead delete path.** Either wire `CheckDaryaftU.Delete_Check` to the button or remove the button (§2.3 T3). | The UI stops lying. | Users press "Delete cheque" and nothing happens, with no message. |
| A7 | **Fix `TankhahList.B_NewClick`'s cross-module `Locate` on `checkeditF.Tag`** (§7.7 defect 1). | Grid positions correctly after a save. | Cosmetic. |
| A8 | **Add `M_Coid` to the delete predicate in `Delete_Check` and fix the missing space** producing `M_Coid=1403and` (§10.2, §10.10). | The statement becomes syntactically valid. | The method is currently unreachable; it would fail the moment it were wired up. |

### 13.B Data-model proposals

| # | Proposal | Rationale |
|---|---|---|
| B1 | **Store dates as `DATE` (Gregorian) with Jalali rendered in the UI.** Keep the original string in a `*_jalali_raw` column through the migration and flag unparseable rows rather than guessing (§5.6). | String dates are correct only by convention; one malformed row corrupts sorting and filtering silently and undetectably. |
| B2 | **Drop every `*Name` / `*CR` denormalised column** (`S_BesName`, `S_BedCR`, `CM_CodeName`, `TD_BedName`, …) and join to `accounts` instead. | They are inconsistent today — `S_BesName` is 100 chars and holds the last segment, `S_BedName` is 50, `TD_BedName` holds the full path, `TM_CodeName` the last segment (§1.6). |
| B3 | **Drop `StateName`; derive the label from the status enum** (§1.1). | It is written by four different screens with four different strings for overlapping states. |
| B4 | **Drop `S_Zssn` / `S_ZCR` / `S_ZName`** unless §12 Q6 shows historical data (§4.5). | Dead columns with no history preserve nothing. |
| B5 | **Add real event timestamps.** `DCheck2` has `S_Date` (an operator-entered Jalali date) and no `created_at`; `DCheck` has no `created_at`/`updated_at`; `S_UserID` is overwritten on edit so "created by" is lost (§1.1, §1.2). | There is currently no way to answer "when was this actually entered, and by whom".<br>See also `docs/08-platform-and-security.md`: the system has no audit trail at all. |
| B6 | **Give the cheque the fields it lacks**: issuing bank, branch, drawer account number, drawer name. Today they can only be smuggled into `S_Desc` (§1.1). | A cheque without its bank cannot be reconciled against a bank statement. |
| B7 | **Record the deposit's target bank on the cheque, not only in the event log** (§11.3). | "Which bank is holding this cheque right now" is currently unanswerable without reading `DCheck2`. |
| B8 | **Model the physical bank / bank account as a first-class entity**, using `BankTanzim`'s `BN_*` sketch as the requirements (§1.7, §11.15). | Bank accounts are currently indistinguishable from any other chart-of-accounts leaf. |
| B9 | **Give `CheckDetail`'s `CD_Jari` and `CD_BankNo` honest names** (`payee_account_holder_name`, `payee_bank_account_number`) and validate the IBAN with the existing `IS_ShabaNo` logic (§1.5). | Two fields whose legacy names say nothing about their contents. |
| B10 | **Constrain `S_DateS` to the same width as `S_Date`.** It is `varchar(50)` holding a 10-character date (§1.1). | Trailing whitespace breaks both `ORDER BY` and the aging filter. |

### 13.C Behaviour and validation proposals

| # | Proposal | Rationale |
|---|---|---|
| C1 | **Validate the due date**: parseable, and ≥ the receipt date. Decide separately whether it must be inside the fiscal year (§12 Q8). | Today it is validated not at all and defaults to *tomorrow* (§5.3). |
| C2 | **Require and de-duplicate cheque numbers** per the answer to §12 Q9. | Two identical cheque numbers from the same drawer are silently accepted. |
| C3 | **Check `S_Bank.Tag > 0` before saving a collection** (§9.4 / §11.5). | The only unvalidated account picker in the module; posts to account id 0. |
| C4 | **Check `RecordCount` after every control-account lookup** in `CheckBargashtu` / `CheckVosoolU` (§10.5). | A missing per-counterparty control account currently produces a posting against account 0 with no error. |
| C5 | **Move validation to the server.** Today every rule is a client-side dialog with no database constraint behind it (§9). | In a Rust + React rebuild the client cannot be the enforcement point at all. |
| C6 | **Make each transition atomic.** Today the state change and the posting are in two separate SQL transactions in the same batch, with no rollback anywhere (§8.5 defect 6, §10.12). | A partial failure leaves a cheque in a new state with no accounting entry. |
| C7 | **Enforce debit = credit at posting time**, not only on the voucher's `0 → 1` transition. | Treasury currently relies entirely on a later check in a different module. |

### 13.D UI proposals

| # | Proposal | Rationale |
|---|---|---|
| D1 | **Implement the filters that were built and left unwired**: by state, by counterparty, by due date (§5.5). The SQL already exists in `CheckListDU.ReopenQ1`. | The cheque list currently shows every cheque of every year with no way to narrow it. |
| D2 | **Scope the cheque and deposit lists to the selected fiscal year**, as the issued-cheque and petty-cash lists already are (§3.4). Or make the year an explicit, visible filter. | The four "wrong fiscal year" error dialogs exist only because the list mixes years. |
| D3 | **Restore the state colour-coding** that is written and disabled by an `Exit;` (§2.1). | It was designed; the colour map is still in the source. |
| D4 | **Show due-date urgency**: overdue / due within N days, as a computed column and a badge. | There is no aging concept beyond a single `<=` cutoff, and no alerts of any kind (§5.5). |
| D5 | **Resolve `S_UserID` to a user name** in the history grid (§11.1). | It currently displays a raw integer. |
| D6 | **Fix the two wrong window titles** (`CheckListDU`, `CheckListU` both say "bank settings") and the misspelt `درریافت چک` (§11.1, §11.2). | |
| D7 | **Server-side pagination on all four lists.** Every one of them does `SELECT *` with no `LIMIT` (§10.9). | |
| D8 | **Move per-user grid/window preferences from the INI file to the user profile** so they follow the user across machines. | |
| D9 | **Un-bake the company name from the petty-cash report** and read it from the fiscal-year settings row as every other report does (§11.13). | |

### 13.E Things deliberately *not* proposed

- **Do not** invent an issued-cheque lifecycle (notes payable, own-cheque clearing). The business may
  genuinely not track it — §12 Q7 must be answered first.
- **Do not** merge `DCheck` and `CheckMaster` into one `cheques` table. They are different documents
  with different cardinality (§3.2); merging would force a nullable half-schema on both.
- **Do not** port `Ghabz.pas` into treasury — it is a weighbridge ticket and belongs to inventory
  (§6.6).
- **Do not** port `Asnad_Daryaft_NewU`, `BankTanzim`'s implementation, `RP1_Old`,
  `New_From_Factor`, `Edit_From_Factor`, `MakeSanad_FishVariz`, or `TUtil.FarsiDate`. All are dead.


---

[← 12. Open questions](06-12-open-questions.md) | [index](00-index.md) | [14. Naming map →](06-14-naming-map.md)
