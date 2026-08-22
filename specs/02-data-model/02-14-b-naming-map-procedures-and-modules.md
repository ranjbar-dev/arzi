_Part of [02-data-model](../02-data-model.md) — [index](00-index.md)_

### 14.9 `DCheck` → `cheques`

| Legacy | Proposed | | Legacy | Proposed |
|---|---|---|---|---|
| `S_SSN` | `id` | | `S_BedSSN` | `notes_receivable_account_id` |
| `S_COID` | `fiscal_year_id` | | `S_BedCR` | **drop** |
| `S_State` | `status` | | `S_BedName` | **drop** |
| `S_StateName` | **drop** | | `S_Zssn` | **drop** (dead — never read or written) |
| `S_CheckNo` | `cheque_number` | | `S_ZCR` | **drop** (dead) |
| `S_Sanad` | `voucher_number` | | `S_ZName` | **drop** (dead) |
| `S_Date` | `received_date` | | `S_linkPrg` | `source_module` |
| `S_DateS` | `due_date` (**50 chars — §12.2**) | | `S_LinkSSN` | `source_id` |
| `S_Mab` | `amount` | | `S_UserID` | `created_by` (really "last editor") |
| `S_Desc` | `description` | | | |
| `S_BesSSN` | `payer_account_id` | | | |
| `S_BesCR` | **drop** | | | |
| `S_BesName` | **drop** | | | |

`S_State` → `cheque_status`: `1` → `in_hand` (`چک موعدي در صندوق`) **and** `bounced`
(`چک برگشت شده از بانک`) — **overloaded, → §13.12**; `2` → `at_bank` (`چک موعدی در بانک`);
`3` → *(never written)*; `4` → `returned_to_issuer` (`چک مسترد شد`); `5` → `cleared`
(`چک وصول شد`).

### 14.10 `DCheck2` → `cheque_events`

| Legacy | Proposed | | Legacy | Proposed |
|---|---|---|---|---|
| `S_SSN` | `id` | | `S_State` | `status_after` (wrong for bounces — `06-treasury.md` §2.1) |
| `S_Link` | `cheque_id` | | `S_StateName` | **drop** |
| `S_COID` | `fiscal_year_id` | | `S_BedSSN` | `debit_account_id` (declared `TStringField`? §12.5) |
| `S_Sanad` | `voucher_number` | | `S_BesSSN` | `credit_account_id` (null for `S_State = 5`) |
| `S_Date` | `event_date` | | `S_Desc` | `description` |
| `S_Mab` | `amount` | | `S_UserID` | `created_by` |

### 14.11 `DFish` → `deposit_slips`

| Legacy | Proposed | | Legacy | Proposed |
|---|---|---|---|---|
| `S_SSN` | `id` | | `S_BesCR` | **drop** |
| `S_COID` | `fiscal_year_id` | | `S_BesName` | **drop** |
| `S_State` | `deposit_method` (**a channel, not a lifecycle**) | | `S_BankSSN` | `bank_account_id` |
| `S_StateName` | **drop** | | `S_BankCR` | **drop** |
| `S_FishNo` | `slip_number` | | `S_BankName` | **drop** |
| `S_Sanad` | `voucher_number` | | `S_UserID` | `created_by` |
| `S_Date` | `deposit_date` | | `S_LinkPRG` | `source_module` |
| `S_Mab` | `amount` | | `S_LinkSSN` | `source_id` |
| `S_Desc` | `description` | | `S_DateS` | **?** — read at `FISHDaryaftU.pas:178`, declared nowhere (§12.2) |
| `S_BesSSN` | `payer_account_id` | | | |

### 14.12 `CheckMaster` / `CheckDetail` → `cheque_payment_documents` / `cheque_payment_lines`

| Legacy | Proposed | | Legacy | Proposed |
|---|---|---|---|---|
| `CM_SSN` | `id` | | *(unknown identity)* | `id` |
| `CM_Coid` | `fiscal_year_id` | | `CD_CMSSN` | `document_id` |
| `CM_No` | `batch_number` | | `CD_Coid` | `fiscal_year_id` |
| `CM_Sanad` | `voucher_number` | | `CD_Bed` | `payee_account_id` |
| `CM_Date` | `issue_date` | | `CD_BedCR` | **drop** |
| `CM_Mab` | `total_amount` (computed from lines) | | `CD_BedName` | **drop** |
| `CM_Desc` | `description` | | `CD_Mab` | `amount` |
| `CM_Tittle` *(sic)* | `letter_body` | | `CD_Desc` | `description` |
| `CM_Code` | `bank_account_id` | | `CD_BankNo` | `payee_bank_account_number` (26 = IBAN length) |
| `CM_CodeCR` | **drop** | | `CD_Jari` | `payee_account_holder_name` (name is misleading — `جاری` = "current") |
| `CM_CodeName` | **drop** | | | |
| `CM_Count` | `line_count` | | | |
| `CM_UserID` | `created_by` | | | |

### 14.13 `TankhahMaster` / `TankhahDetail` → `petty_cash_documents` / `petty_cash_lines`

| Legacy | Proposed | | Legacy | Proposed |
|---|---|---|---|---|
| `TM_SSN` | `id` | | *(unknown identity)* | `id` |
| `TM_Coid` | `fiscal_year_id` | | `TD_TMSSN` | `document_id` |
| `TM_No` | `claim_number` | | `TD_Coid` | `fiscal_year_id` |
| `TM_Sanad` | `voucher_number` | | `TD_Bed` | `expense_account_id` |
| `TM_Date` | `claim_date` | | `TD_BedCR` | **drop** |
| `TM_Mab` | `total_amount` | | `TD_BedName` | **drop** (holds the **full** path, unlike the header's last segment) |
| `TM_Desc` | `description` | | `TD_Mab` | `amount` |
| `TM_Code` | `custodian_account_id` (`تنخواه‌دار`) | | `TD_Desc` | `description` |
| `TM_CodeCR` | **drop** | | | |
| `TM_CodeName` | **drop** | | | |
| `TM_Count` | `line_count` | | | |
| `TM_UserID` | `created_by` | | | |

There is no `TM_Tittle` and no `TD_BankNo`/`TD_Jari` — the covering-letter block and the two
banking-reference columns exist only on the cheque batch (`06-treasury.md` §1.6).

### 14.14 `Anbar_Factor` / `Anbar_FactorD` → `inventory_invoices` / `inventory_invoice_lines`

| Legacy | Proposed | | Legacy | Proposed |
|---|---|---|---|---|
| `AF_SSN` | `id` | | `AFD_SSN` | `id` |
| `AF_COID` | `fiscal_year_id` | | `AFD_Coid` | `fiscal_year_id` |
| `AF_Factor` | `invoice_number` | | `AFD_Factor` | `invoice_id` (legacy: number + year) |
| `AF_Type` | `document_type` | | `AFD_Type` | **drop** (copy of header) |
| `AF_Date` | `invoice_date` | | `AFD_Date` | **drop** (copy of header) |
| `AF_Customer` | `counterparty_account_id` | | `AFD_Customer` | **drop** (back-filled copy, `Anbar_Amalkard.pas:168`) |
| `AF_Sanad` | `voucher_number` | | `AFD_Code` | `item_id` |
| `AF_Mab` | `subtotal` (`SUM(AFD_Kol)`) | | `AFD_Name` | **drop** (denormalised item name) |
| `AF_Kasr` | `total_deduction` | | `AFD_Prop` | `specification` |
| `AF_Maliat` | `total_tax` | | `AFD_Vahed` | `unit_of_measure` |
| `AF_Total` | `total_amount` (`SUM(AFD_Total)`) | | `AFD_Vahed2`, `AFD_Vahed3` | `unit_of_measure_2/3` **?** |
| `AF_Desc` | `description` | | `AFD_Num` | `quantity` |
| `AF_Desc1`…`AF_Desc5` | `settlement_note_1…5` **?** | | `AFD_Phi` | `unit_price` |
| `AF_Date1`…`AF_Date5` | `settlement_date_1…5` **?** | | `AFD_Kol` | `line_gross` |
| `AF_Mab1`…`AF_Mab5` | `settlement_amount_1…5` **?** | | `AFD_Kasr` | `line_deduction` |
| `AF_Sel2`…`AF_Sel5` | `settlement_selected_2…5` **?** | | `AFD_Maliat` | `line_tax` |
| `AF_CustomerN` | **drop** (computed in SQL) | | `AFD_Total` | `line_total` |
| `Af_typeN` | **drop** (`CASE` expression, `AnbarListU.pas:540`) | | `AFD_TypeN`, `AFD_IN`, `AFD_OUT` | **drop** (computed) |

⚠ The `AF_Desc1..5` / `AF_Date1..5` / `AF_Mab1..5` / `AF_Sel2..5` repeating groups are a
**five-slot settlement array flattened into columns** (they surface in the `Anbar_Tasfieh`
settlement query, `Dmu.dfm:1017-1110`). The proposed names above are **inferences and must be
confirmed** (§12.5); the correct target is a child table `inventory_invoice_settlements`, which is
a structural change — **→ §13** if adopted.

`AF_Type` → `inventory_document_type`: `1` → `goods_receipt` (`رسید انبار`),
`2` → `goods_issue` (`حواله انبار`), `3`–`9` unlabelled (§12.9).

### 14.15 `Anbar_Jens` → `items`, `Anbar_Config` → `warehouses`, `Anbar_Vahed` → `units_of_measure`

| Legacy | Proposed | Notes |
|---|---|---|
| `AJ_Code` | `code` | business key, manually assigned |
| `AJ_ID` | `warehouse_id` | FK → `warehouses` |
| `AJ_Name` | `name` | |
| `AJ_Prop` | `specification` | |
| `AJ_Vahed` | `unit_of_measure` | free-text copy |
| `AJ_VahedC` | `unit_of_measure_id` | FK → `units_of_measure` (`AnbarCalaAddU.pas:160`) |
| `AJ_Vahed2`, `AJ_Vahed3` | `unit_of_measure_2/3` **?** | |
| `AJ_Phi` | `default_unit_price` | `bigint` rial |
| `AJ_PhiS` | **drop?** | computed in some result sets |
| `AJ_Maliat` | `is_taxable` | `0`/`1`; forces VAT to 0 when 0 (`AnbarFactorAddU.pas:145-146`) |
| `AJ_Manfi` | `allow_negative_stock` | `0`/`1`; checked at `AnbarFactorAddU.pas:177` |
| `AJ_Alarm` | `reorder_level` | shown as `rem2` (`AnbarFactorAddU.pas:133`) |
| `AJ_UserID` | `updated_by` | |
| `AJ_DateTime` | `updated_at` | written `GetDate()` (`AnbarCalaAddU.pas:169`) |
| `AJ_Net` | `updated_from_host` | the workstation name (`Util.GetComputerName`, `AnbarCalaAddU.pas:161`) |
| `SSTID` | `tax_system_item_code` | 13 chars — the Iranian tax portal's item code (`AnbarCalaAddU.pas:162`) |

| Legacy (`Anbar_Config`) | Proposed (`warehouses`) |
|---|---|
| *(key column not visible in source — §12.15)* | `id` |
| `AC_Name` | `name` |
| `AC_DMaliat` | `default_tax_rate` |
| `AC_Kharid` | `purchase_account_id` (`خرید`) |
| `AC_BKharid` | `purchase_return_account_id` (`برگشت خرید`) |
| `AC_Foroosh` | `sales_account_id` (`فروش`) |
| `AC_BForoosh` | `sales_return_account_id` (`برگشت فروش`) |
| `AC_Kasr` | `deduction_account_id` (`کسر`) |
| `AC_Maliat` | `tax_account_id` (`مالیات`) |

| Legacy (`Anbar_Vahed`) | Proposed (`units_of_measure`) |
|---|---|
| `AV_Code` | `id` |
| `AV_Name` | `name` |

### 14.16 `Tanzim` → `app_settings`

| Legacy | Proposed |
|---|---|
| `T_ID` | `key` (numeric id kept only in the migration mapping) |
| `T_Str` | `value` |
| `T_Int` | **drop** (always `'0'`, never read — §8.2) |
| `T_Desc` | `label_fa` |
| *(new)* | `value_type` |

| `T_ID` | Proposed key | Persian |
|---|---|---|
| 1001–1004 | `invoice.signature_1` … `invoice.signature_4` | `فاکتور امضا ۱-۴` |
| 1005–1006 | `invoice.heading_1`, `invoice.heading_2` | `فاکتور عنوان ۱-۲` |
| 1007 | `invoice.counterparty_label` | `طرف حساب` |
| 1008 | `invoice.show_amount` | `نمایش مبلغ` |
| 1009 | `invoice.show_discount` | `نمایش تخفیف` |
| 1010 | `invoice.show_tax` | `نمایش مالیات` |
| 1011–1014 | `voucher.signature_1` … `voucher.signature_4` | `سند امضا ۱-۴` |
| 1015 | `invoice.official_footer` | `پانویس فاکتور رسمی` |

### 14.17 `PassWord` → `users`, `Pass_Config` → `user_permissions`

| Legacy | Proposed |
|---|---|
| `UserCode` | `id` |
| `UserName` | `username` |
| `Password` | `password_hash` (**plaintext today** — §13.17) |
| `Enabled` | `is_active` |
| `Supervisor` | `is_superuser` |
| `P_User` | `user_id` |
| `P_ID` | `permission_id` |
| `P_DESC` | **drop** (denormalised Persian checkbox caption) |

### 14.18 Stored procedures → Rust services / API endpoints

| Legacy procedure | Kind | Proposed Rust unit | Proposed API |
|---|---|---|---|
| `MoeinAdd` | write | `accounting::voucher_line_service::create` | `POST /api/v1/vouchers/{id}/lines` |
| `MakeSanad_CheckDaryafti` | write | `treasury::posting::post_cheque_receipt` | internal — invoked by `POST /api/v1/cheques` |
| `Anbar_AddToFactor` | write | `inventory::invoice_service::add_line` | `POST /api/v1/inventory-invoices/{id}/lines` |
| `Sahamdar_Edit` | write | `parties::party_service::upsert` | `POST` / `PUT /api/v1/parties` |
| `Sarfasl_ADD` | write | `accounting::account_service::create` | `POST /api/v1/accounts` |
| `Sarfasl_Deep` | write | `accounting::account_service::delete_checked` | `DELETE /api/v1/accounts/{id}` |
| `Active_Set` | maintenance | **deleted** — `is_active`/`child_count` become derived or trigger-maintained | — |
| `XNew` | read | **deleted** — `jalali::today()` in Rust, DB clock via `now()` | — |
| `Taraz4Setooni` | read | `reporting::trial_balance::four_column` | `GET /api/v1/reports/trial-balance?columns=4` |
| `Taraz_6Sotooni` | read | `reporting::trial_balance::six_column` | `GET /api/v1/reports/trial-balance?columns=6` |
| `Moein_View_Daftar` | read | `reporting::subsidiary_ledger` | `GET /api/v1/reports/ledger` |
| `Moein_All` | read | `accounting::year_end::account_balances` | internal (year-end close) |
| `MoeinViewSanad` | read | `accounting::voucher_service::get_lines` | `GET /api/v1/vouchers/{id}/lines` |
| `MoeinTotalSanad` | read | folded into `get_voucher` | `GET /api/v1/vouchers/{id}` |
| `Moein_ChapSanad` | read | `reporting::voucher_print` | `GET /api/v1/vouchers/{id}/print` |
| `Asnad_View` | read | `accounting::voucher_service::list` | `GET /api/v1/vouchers` |
| `KolState` | read | `reporting::gl_account_state` | `GET /api/v1/reports/gl-state` |
| `Sarfasl_view` | read | `accounting::account_service::list` | `GET /api/v1/accounts` |
| `Sarfasl_Seek_SSN` | read | `account_service::get` | `GET /api/v1/accounts/{id}` |
| `Sarfasl_Seek_Name` | read | `account_service::search` | `GET /api/v1/accounts?q=` |
| `Select_Kol` / `Select_moein` / `Select_Taf1` / `Select_Taf2` | read | one function | `GET /api/v1/accounts?parent=<code>` |
| `Sahamdar_Seek` | read | `party_service::get_by_card` | `GET /api/v1/parties?card=` |
| `Sahamdar_Show` | read | `party_service::get` | `GET /api/v1/parties/{id}` |
| `Anbar_AjnasView` | read | `inventory::item_service::list` | `GET /api/v1/items?warehouse=` |
| `Anbar_CardJensi` | read | `inventory::stock_card` | `GET /api/v1/reports/stock-card` |
| `Anbar_Mandeh` | read | `inventory::stock_on_hand` | `GET /api/v1/reports/stock-on-hand` |
| `Anbar_PrintFactor` | read | `reporting::invoice_print` | `GET /api/v1/inventory-invoices/{id}/print` |
| `Anbar_ReportKharidForoosh` | read | `reporting::purchase_sales` | `GET /api/v1/reports/purchase-sales` |
| `B_SelectSerial` | read (**external**) | `integrations::rppc::lookup_receipt` | do **not** port — integrate (§5.4) |

### 14.19 Server-side functions → Rust

| Legacy | Proposed |
|---|---|
| `dbo.Noto3(bigint)` | frontend `Intl.NumberFormat('fa-IR')`; **no** server-side formatter (§7.7) |
| `dbo.Make_L(...)` | **deleted** — ordering by `(general_ledger_code, subsidiary_code, analytic1_code, analytic2_code)` or a recursive CTE (§13.6) |
| `dbo.Make_R(...)` | **deleted** — same |
| `master.dbo.xp_fileexist` | **deleted** — no host filesystem access (§10.8) |

### 14.20 Delphi data-module members → Rust modules

| Legacy `TDM` member | Proposed |
|---|---|
| `TDM.Current_Date` (`Dmu.pas:1232`) | `clock::today_tehran()` |
| `TDM.MiladiToShamsi` (`Dmu.pas:362`) | `jalali::from_gregorian()` — **dead code today** (§6.3) |
| `TUtil.FarsiDate` / `DecodedateF` (`Utility.pas:413`) | same; **dead code today** |
| `TDM.isValidDate` (§6.4) | `jalali::parse()` returning `Result` |
| `TDM.inttoStr3` (`Dmu.pas:859`) | frontend formatting only |
| `TDM.Str2String` (`Dmu.pas:604`) | `money::to_persian_words()` — with full `i64` scale coverage (§7.5) |
| `TDM.IsValidShaba` (`Dmu.pas:196`) | `validation::iban()` (ISO 13616 mod-97) |
| `TDM.IsEnabel` *(sic)* (`Dmu.pas:1552`) | `authz::has_permission()` — enforced **server-side** (§13.17) |
| `TDM.New_Sanad` (`Dmu.pas:1247`) | `numbering::next_voucher_number()` (§5.7) |
| `TDM.New_AnbarFactor` (`Dmu.pas:1258`) | `numbering::next_invoice_number()` |
| `TDM.Get_NewSanad_DateID` (§5.3.2) | `numbering::find_or_create_daily_draft()` |
| `TDM.DMoein_Make` (`Dmu.pas:828`) | `accounting::voucher_service::upsert_header` |
| `TDM.Dmoein_UpdateMab` | **deleted** — totals derived or maintained in the same transaction |
| `TDM.Update_Sarfasl_Child` (`Dmu.pas:300`) | **deleted** — derived (§13.6) |
| `TDM.is_Sarfasl_Last_Deep` (`Dmu.pas:920`) | `account_service::is_leaf()` — **fail-closed** (§9.6) |
| `TDM.Is_Admin_Or_Valid_Daftar` / `_Jari` / `_Sanad` (`Dmu.pas:920-1014`) | `authz::can_post_to()` — one implementation, fail-closed |
| `TDM.Is_New_Sanad_Valid` (`Dmu.pas:1008`) | `fiscal_year::assert_open()` |
| `TDM.Get_paramstr` / `Set_paramstr` (`Dmu.pas:468-508`) | `settings::get::<T>()` / `set()` — typed, seeded, no lazy creation (§8.6) |
| `TDM.SanDoogh_k` / `_M` / `_KM` (`Dmu.pas:1065`) | `system_accounts::cash()` |
| `TDM.Jaryan_K` / `_M` / `_KM` (`Dmu.pas:1100`) | `system_accounts::cheques_in_transit()` |
| `TDM.GetReg_String` / `SetReg_String` (`Dmu.pas:545`) | **deleted** — never called; no registry (§8.5) |
| `TMyIni` / `TPropSaveFile` (§8.1) | `user_preferences` table + `DATABASE_URL` env var (§8.6) |
| `TSysInfo` (`LockUnit.pas`) | **deleted** — machine-fingerprint licensing dropped (§8.6, §13) |

---

[← 02-14-a-naming-map-tables-and-columns.md](02-14-a-naming-map-tables-and-columns.md)
