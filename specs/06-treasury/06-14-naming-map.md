_Part of [06-treasury](../06-treasury.md) — [index](00-index.md)_

## 14. Naming map

Conventions per `docs/01-glossary.md` §7: PostgreSQL `snake_case` plural tables / singular columns;
Rust `snake_case` fields and `PascalCase` types in a `treasury` module; TypeScript `PascalCase`
components and `camelCase` props; API routes `/api/v1/<kebab-plural>`.

### 14.1 Tables

| Legacy | Proposed table | Rust type | API resource |
|---|---|---|---|
| `DCheck` | `received_cheques` | `ReceivedCheque` | `/api/v1/received-cheques` |
| `DCheck2` | `received_cheque_events` | `ReceivedChequeEvent` | `/api/v1/received-cheques/{id}/events` |
| `DFish` | `deposit_slips` | `DepositSlip` | `/api/v1/deposit-slips` |
| `CheckMaster` | `cheque_payment_batches` | `ChequePaymentBatch` | `/api/v1/cheque-payment-batches` |
| `CheckDetail` | `cheque_payment_batch_lines` | `ChequePaymentBatchLine` | *(nested)* |
| `TankhahMaster` | `petty_cash_claims` | `PettyCashClaim` | `/api/v1/petty-cash-claims` |
| `TankhahDetail` | `petty_cash_claim_lines` | `PettyCashClaimLine` | *(nested)* |
| `TCheck` | *(drop — never used, §1)* | — | — |
| `BN_*` *(inert, §1.7)* | `banks` | `Bank` | `/api/v1/banks` |

### 14.2 `DCheck` → `received_cheques`

| Legacy | Proposed | Note |
|---|---|---|
| `S_SSN` | `id` | |
| `S_COID` | `fiscal_year_id` | fiscal year, **not** company |
| `S_State` | `status` | enum, §14.7 |
| `S_StateName` | *drop* | derived from `status` |
| `S_CheckNo` | `cheque_number` | |
| `S_Sanad` | `voucher_number` | of the *receipt* posting only |
| `S_Date` | `received_on` (+ `received_on_jalali_raw`) | |
| `S_DateS` | `due_date` (+ `due_date_jalali_raw`) | |
| `S_Mab` | `amount` | |
| `S_Desc` | `description` | |
| `S_BesSSN` / `S_BesCR` / `S_BesName` | `payer_account_id` / *drop* / *drop* | |
| `S_BedSSN` / `S_BedCR` / `S_BedName` | `notes_receivable_account_id` / *drop* / *drop* | |
| `S_Zssn` / `S_ZCR` / `S_ZName` | *drop* | dead, §4 |
| `S_linkPrg` | `source_module` | enum, §14.8 |
| `S_LinkSSN` | `source_id` | |
| `S_UserID` | `updated_by` | plus new `created_by`, `created_at`, `updated_at` |

### 14.3 `DCheck2` → `received_cheque_events`

| Legacy | Proposed |
|---|---|
| `S_SSN` | `id` |
| `S_Link` | `received_cheque_id` |
| `S_COID` | `fiscal_year_id` |
| `S_Sanad` | `voucher_number` |
| `S_Date` | `event_date` (+ `event_date_jalali_raw`) |
| `S_Mab` | `amount` |
| `S_State` | `status_after` |
| `S_StateName` | *drop* |
| `S_BedSSN` | `debit_account_id` |
| `S_BesSSN` | `credit_account_id` |
| `S_Desc` | `description` |
| `S_UserID` | `created_by` (+ new `created_at`) |
| *(new)* | `event_type` — `Deposited` / `Bounced` / `Cleared` / `Returned`, today only inferable from `M_Id` + `S_StateName` |

### 14.4 `DFish` → `deposit_slips`

| Legacy | Proposed |
|---|---|
| `S_SSN` | `id` |
| `S_COID` | `fiscal_year_id` |
| `S_State` | `deposit_method` — enum, §14.9 (**not** a status) |
| `S_StateName` | *drop* |
| `S_FishNo` | `slip_number` |
| `S_Sanad` | `voucher_number` |
| `S_Date` | `deposited_on` (+ `deposited_on_jalali_raw`) |
| `S_Mab` | `amount` |
| `S_Desc` | `description` |
| `S_BesSSN` / `S_BesCR` / `S_BesName` | `payer_account_id` / *drop* / *drop* |
| `S_BankSSN` / `S_BankCR` / `S_BankName` | `bank_account_id` / *drop* / *drop* |
| `S_UserID` | `updated_by` |
| `S_LinkPRG` / `S_LinkSSN` | `source_module` / `source_id` |
| `S_DateS` *(existence unconfirmed, §12 Q3)* | — |

### 14.5 `CheckMaster` / `CheckDetail` → `cheque_payment_batches` / `_lines`

| Legacy | Proposed |
|---|---|
| `CM_SSN` | `id` |
| `CM_Coid` | `fiscal_year_id` |
| `CM_No` | `batch_number` |
| `CM_Sanad` | `voucher_number` |
| `CM_Date` | `issued_on` (+ `issued_on_jalali_raw`) |
| `CM_Mab` | `total_amount` *(computed from lines)* |
| `CM_Desc` | `description` |
| `CM_Tittle` | `letter_body` |
| `CM_Code` / `CM_CodeCR` / `CM_CodeName` | `bank_account_id` / *drop* / *drop* |
| `CM_Count` | *drop — derive* |
| `CM_UserID` | `updated_by` |
| `CD_CMSSN` | `batch_id` |
| `CD_Coid` | *drop — inherited from the header* |
| `CD_Bed` / `CD_BedCR` / `CD_BedName` | `payee_account_id` / *drop* / *drop* |
| `CD_Mab` | `amount` |
| `CD_Desc` | `description` |
| `CD_BankNo` | `payee_bank_account_number` |
| `CD_Jari` | `payee_account_holder_name` |

### 14.6 `TankhahMaster` / `TankhahDetail` → `petty_cash_claims` / `_lines`

| Legacy | Proposed |
|---|---|
| `TM_SSN` | `id` |
| `TM_Coid` | `fiscal_year_id` |
| `TM_No` | `claim_number` |
| `TM_Sanad` | `voucher_number` |
| `TM_Date` | `claimed_on` (+ `claimed_on_jalali_raw`) |
| `TM_Mab` | `total_amount` *(computed)* |
| `TM_Desc` | `description` |
| `TM_Code` / `TM_CodeCR` / `TM_CodeName` | `custodian_account_id` / *drop* / *drop* |
| `TM_Count` | *drop — derive* |
| `TM_UserID` | `updated_by` |
| `TD_TMSSN` | `claim_id` |
| `TD_Coid` | *drop* |
| `TD_Bed` / `TD_BedCR` / `TD_BedName` | `expense_account_id` / *drop* / *drop* |
| `TD_Mab` | `amount` |
| `TD_Desc` | `description` |

### 14.7 `DCheck.S_State` → `ChequeStatus`

| Legacy code | Persian | Proposed variant |
|---|---|---|
| 1 (`چک موعدي در صندوق`) | dated cheque in the cash box | `InHand` |
| 1 (`چک برگشت شده از بانک`) | bounced back from the bank | `Bounced` — **needs a new code**, §13 A1 |
| 2 | `چک موعدی در بانک` | `AtBank` |
| 3 | *(never written)* | — |
| 4 | ` چک مسترد شد ` | `ReturnedToIssuer` |
| 5 | `چک وصول شد` | `Cleared` |

### 14.8 `S_linkPrg` / `S_LinkPRG` → `SourceModule`

| Value | Persian label | Proposed variant | Seen on |
|---|---|---|---|
| 0 | *(blank)* | `Manual` | both |
| 1 | ` فاکتور ` / ` فاکتور کالا ` | `GoodsInvoice` | `DCheck`, `DFish` |
| 2 | ` فاکتور پسته ` | `PistachioInvoice` | `DFish` only |

### 14.9 `DFish.S_State` → `DepositMethod`

| Value | Persian | Proposed variant |
|---|---|---|
| 1 | `واریزی از طریق کارتخوان` | `PosTerminal` |
| 2 | `واریزی از طریق فیش نقدی` | `CashSlip` |
| 3 | `واریزی از طریق کارت به کارت` | `CardToCard` |
| 4 | `واریز حواله پایا و ساتنا` | `WireTransfer` |

### 14.10 `Moein.M_Id` → `TreasuryPostingKind`

| `M_Id` | Proposed variant |
|---|---|
| 21 | `ChequeReceived` |
| 22 | `ChequeDeposited` / `ChequeBounced` — **must be split**, they share the code today (§8.1) |
| 23 | `ChequeCleared` |
| 24 | `ChequeReturned` |
| 25 | `DepositSlip` |
| 26 | `ChequePaymentBatch` |
| 41 | `PettyCashClaim` |

### 14.11 Units → modules and components

| Legacy unit / form | Proposed Rust module / handler | Proposed React component | Route |
|---|---|---|---|
| `CheckListDU` / `TCheckListDF` | `treasury::received_cheques::list` | `ReceivedChequeList` | `/treasury/received-cheques` |
| `CheckDaryaftU` / `TCheckDaryaftF` | `treasury::received_cheques::{create,update}` | `ReceivedChequeForm` | `/treasury/received-cheques/new`, `/:id/edit` |
| `CheckDaryaft2U` / `TCheckDaryaft2F` | `treasury::received_cheques::deposit` | `ChequeDepositDialog` | `/:id/deposit` |
| `CheckBargashtu` / `TCheckBargashtF` | `treasury::received_cheques::bounce` | `ChequeBounceDialog` | `/:id/bounce` |
| `CheckVosoolU` / `TCheckVosoolF` | `treasury::received_cheques::clear` | `ChequeClearDialog` | `/:id/clear` |
| `CheckEsterdadU` / `TCheckEsterdadF` | `treasury::received_cheques::return_to_issuer` | `ChequeReturnDialog` | `/:id/return` |
| `FishListD` / `TFishListDF` | `treasury::deposit_slips::list` | `DepositSlipList` | `/treasury/deposit-slips` |
| `FISHDaryaftU` / `TFishDaryaftF` | `treasury::deposit_slips::{create,update}` | `DepositSlipForm` | `/treasury/deposit-slips/new`, `/:id/edit` |
| `CheckListU` / `TCheckListF` | `treasury::payment_batches::list` | `ChequePaymentBatchList` | `/treasury/cheque-payment-batches` |
| `CheckEditU` / `TCheckEditF` | `treasury::payment_batches::{create,update}` | `ChequePaymentBatchForm` | `/treasury/cheque-payment-batches/new`, `/:id` |
| `CheckEditAddU` / `TCheckEditAddF` | *(inline)* | `PayeeLineDialog` | *(modal)* |
| `TankhahList` / `TTankhahListF` | `treasury::petty_cash::list` | `PettyCashClaimList` | `/treasury/petty-cash-claims` |
| `TankhahEdit` / `TTankhahEditF` | `treasury::petty_cash::{create,update}` | `PettyCashClaimForm` | `/treasury/petty-cash-claims/new`, `/:id` |
| `TankhahEditAddu` / `TTankhahEditAddF` | *(inline)* | `ExpenseLineDialog` | *(modal)* |
| `BankTanzim` / `TBankTanzimF` | `treasury::banks` *(new)* | `BankSettings` | `/settings/banks` |
| `TarafU` / `Taraf` | `accounting::accounts::lookup` | `AccountPicker` | *(shared component)* |
| `Ghabz` / `TGhabzF` | *(inventory, not treasury)* | — | — |
| `Asnad_Daryaft_NewU` | *(drop — dead)* | — | — |

### 14.12 Data-module helpers used by treasury

| Legacy | Proposed |
|---|---|
| `Dm.Sandoogh_K` / `_M` / `_KM` | `treasury::config::notes_on_hand_account()` |
| `Dm.Jaryan_K` / `_M` / `_KM` | `treasury::config::notes_in_collection_account()` |
| `Dm.Get_NewSanad_DateID(date, ids)` | `accounting::vouchers::allocate_for_date(date, kinds)` |
| `Dm.Get_SanadMaxTX(n)` | `accounting::vouchers::max_state(n)` |
| `Dm.Get_SanadDateID_Valid(n, date, ids)` | `accounting::vouchers::is_valid_for(n, date, kinds)` |
| `Dm.Moein_Tx(n)` | `accounting::vouchers::state(n)` |
| `Dm.Is_New_Sanad_Valid(coid)` | `accounting::fiscal_years::accepts_new_vouchers(id)` |
| `Dm.DMoein_Make(n, date, desc)` | `accounting::vouchers::upsert_header(...)` |
| `Dm.Dmoein_UpdateMab(n)` | `accounting::vouchers::refresh_total(n)` |
| `Dm.is_Sarfasl_Last_Deep_SSN(id)` | `accounting::accounts::is_leaf(id)` |
| `Dm.Sarfasl_SSN_CODEName(id)` | `accounting::accounts::get(id)` |
| `Dm.IsEnabel(user, key)` | `platform::authz::can(user, permission)` |
| `Dm.inttostr3(n)` | *(front-end number formatting)* |
| `Dm.Str2String(s)` / `TUtil.No2String(n)` / `dbo.Noto3(n)` | `platform::i18n::spell_amount_fa(n)` — **one** implementation, replacing the current three |
| `TUtil.FarsiDate` / `TDM.MiladiToShamsi` / `Tools.TFullDate` | `platform::jalali` — a single maintained library (§5.6) |

### 14.13 Permission keys

| Key | Legacy control | Proposed permission |
|---|---|---|
| 2102 | `S_New` (cheques) / `F_New` (slips) | `treasury.received_cheque.create` / `treasury.deposit_slip.create` — **split; they share a key today** |
| 2103 | `S_Edit` / `F_Edit` | `…update` |
| 2104 | `S_Delete` / `F_Delete` | `…delete` |
| 2105 | `S_Bank` | `treasury.received_cheque.deposit` |
| 2106 | `S_BBank` | `treasury.received_cheque.bounce` |
| 2107 | `S_Vosool` | `treasury.received_cheque.clear` |
| 2108 | `S_DVosool` | `treasury.received_cheque.undo_clear` *(unimplemented)* |
| 2109 | `S_Bargasht` | `treasury.received_cheque.return_to_issuer` |
| 2111 | `B_New` | `treasury.payment_batch.create` |
| 2112 | `B_Edit` | `treasury.payment_batch.update` |
| 2113 | `B_View` | `treasury.payment_batch.read` |
| 2114 | `B_Print1` / `B_Print2` | `treasury.payment_batch.print` |
| 2121 | `B_New` (petty cash) | `treasury.petty_cash_claim.create` |
| 2122 | `B_Edit` | `treasury.petty_cash_claim.update` |
| 2123 | `B_View` | `treasury.petty_cash_claim.read` |
| 2124 | `B_Print1` / `B_Print2` | `treasury.petty_cash_claim.print` |
| 2125 | `B_Delete` (issued cheques) | `treasury.payment_batch.delete` |
| *(none)* | `B_Delete` (petty cash) | `treasury.petty_cash_claim.delete` — **must be added**, §7.6 |
| 1121 | `B_ViewSanad` | `accounting.voucher.read` |

---

[← 13. PROPOSED IMPROVEMENTS (needs user approval)](06-13-proposed-improvements.md) | [index](00-index.md) | _end_
