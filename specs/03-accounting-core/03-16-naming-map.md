_Part of [03-accounting-core](../03-accounting-core.md) — [index](00-index.md)_

## 16. Naming map

Legacy identifier → proposed English name. Follows the conventions in `docs/01-glossary.md` §7:
PostgreSQL `snake_case`, plural tables, `id` primary keys.

### 16.1 Tables

| Legacy table | Meaning | Proposed |
|---|---|---|
| `Sarfasl` | Chart of accounts (all 4 levels) | `accounts` |
| `Moein` | Voucher lines | `voucher_lines` |
| `DMoein` | Voucher headers | `vouchers` |
| `Base` | Fiscal years + company profile + settings | `fiscal_years` (+ `company_settings`) |
| `Password` | Users | `users` |
| `Pass_Config` | Per-user permission grants | `user_permissions` |
| `base_config` | Special-role account registry | `account_roles` |
| `SahamdarConfig` | Party-account subtree registry | `party_account_subtrees` |
| `Tanzim` | Key/value application settings | `settings` |
| `Kinds` | Product grades (pistachio module — **not** account types) | `product_grades` |
| `Anbar.FactorMaster` | Inventory document header | `inventory_documents` |
| `Anbar.FactorKind` | Inventory document types | `inventory_document_types` |
| `Anbar.Anbar` | Warehouse master + its posting accounts | `warehouses` |

### 16.2 `Sarfasl` → `accounts`

| Legacy column | Proposed | Notes |
|---|---|---|
| `S_SSN` | `id` | |
| `S_Ko` | `level1_code` | Kol |
| `S_Mo` | `level2_code` | Moein; `0` above this level |
| `S_Ta1` | `level3_code` | Tafsil 1 |
| `S_Ta2` | `level4_code` | Tafsil 2 |
| `S_Name` | `name` | node name only |
| `S_Child` | `child_count` | or replace with `is_leaf` |
| `S_Lock` | `is_locked` | inherited downward |
| `S_Active` | `is_active` | currently dead — see §14 Q9 |
| `S_Kind` | — | unused; superseded by P1's `account_type` |
| `S_Card` | `party_id` | FK to the party master |
| `S_Bed`, `S_Bes`, `S_Remi`, `S_Count` | *(drop)* | stale denormalisation |
| `M_R`, `M_L`, `FullName`, `LineName` | *(drop, derive)* | display strings |
| `NeedUpdate` | *(drop)* | denormalisation dirty flag |
| `S_Address` | `address` |  |
| `S_Tel` | `phone` |  |
| `S_Fax` | `fax` |  |
| `S_Melli` | `national_id` | کد ملی |
| `S_Sabt` | `registration_number` | شماره ثبت |
| `S_Egh` | `economic_code` | کد اقتصادی |
| `S_Post` | `postal_code` | کد پستی |
| `S_IS_Check`, `S_IS_Fish`, `S_IS_APArdakhti`, `S_IS_ADaryafti` | *(drop)* | superseded by `account_roles` |
| `S_COID` | *(drop or `fiscal_year_id`)* | pending §14 Q3 |

### 16.3 `DMoein` → `vouchers`

| Legacy column | Proposed | Notes |
|---|---|---|
| `DM_SSN` | `id` | |
| `DM_Sanad` | `voucher_number` | unique per fiscal year |
| `DM_Coid` | `fiscal_year_id` | |
| `DM_Date` | `date` + `date_jalali` | |
| `DM_Desc` | `description` | |
| `DM_TX` | `status` | enum `Draft` / `Approved` / `Posted` |
| `DM_Lock` | `is_locked` | independent of `status` |
| `DM_Kind` | `voucher_kind` | enum `Subsidiary` (1) / `Journal` (2) |
| `DM_TBed` | `total_debit` | derived |
| `DM_TBes` | `total_credit` | derived |
| `DM_Count` | `line_count` | derived |
| `DM_Atf` | `folio` | never written — see §14 Q8 |
| `DM_MUser` | `created_by` | note the M/C reversal |
| `DM_MDate` | `created_at` | |
| `DM_CUser` | `updated_by` | |
| `DM_CDate` | `updated_at` | |

### 16.4 `Moein` → `voucher_lines`

| Legacy column | Proposed | Notes |
|---|---|---|
| `M_SSN` | `id` | |
| `M_COID` | `fiscal_year_id` | |
| `M_Sanad` | `voucher_id` | FK, replacing the `(coid, number)` pair |
| `M_Code` | `account_id` | FK to `accounts.id` |
| `M_Ko`, `M_Mo`, `M_Ta1`, `M_Ta2` | *(drop, derive)* | |
| `M_Bed` | `debit` | |
| `M_Bes` | `credit` | |
| `M_Ted` | `quantity` | statistical only |
| `Article` | `description` | note: no `M_` prefix in the legacy schema |
| `M_Date` | *(drop, derive)* | copy of the header date |
| `M_Tx` | *(drop, derive)* | copy of the header status |
| `M_Kind` | *(drop, derive)* | copy of the header kind |
| `M_Id` | `source_kind` | 0 = manual; see §3.4 |
| `M_Link` | `source_id` | source document PK |
| `M_User` | `created_by` | |
| `M_time` | `created_at` | |
| *(new)* | `line_number` | see P2 |

### 16.5 `Base` → `fiscal_years`

| Legacy column | Proposed |
|---|---|
| `CO_ID` | `id` |
| `Co_Name` | `company_name` |
| `Co_Sub` | `system_name` |
| `FromDate` | `start_date` |
| `ToDate` | `end_date` |
| `IsActive` | `is_open` |
| `No_Ko`, `No_Mo`, `No_Ta1`, `No_Ta2` | `level1_digits` … `level4_digits` |
| `Int_Len` | `amount_integer_digits` |
| `Real_Len` | `amount_decimal_digits` |
| `BackupDir` | `backup_directory` |
| `C1081` / `C1081C` | `notes_on_hand_account_id` / `_code` |
| `C1082` / `C1082C` | `notes_in_collection_account_id` / `_code` |
| `Co_Address`, `Co_Tel`, `Co_Fax`, `Co_Web`, `Co_EMail` | `address`, `phone`, `fax`, `website`, `email` |
| `Co_Sabt`, `Co_Melli`, `Co_Egh`, `Co_Post` | `registration_number`, `national_id`, `economic_code`, `postal_code` |
| `Kh1_Code` … `Kh8_Code` / `Kh1_Desc` … | `report_column_N_code` / `_label` (purpose unconfirmed) |

### 16.6 Permissions and users

| Legacy | Proposed |
|---|---|
| `Password.UserCode` | `users.id` |
| `Password.UserName` | `users.username` |
| `Password.password` | `users.password_hash` |
| `Password.Enabled` | `users.is_enabled` |
| `Password.supervisor` | `users.is_supervisor` |
| `Pass_Config.P_User` | `user_permissions.user_id` |
| `Pass_Config.P_ID` | `user_permissions.permission_key` |
| `Pass_Config.P_DESC` | *(drop — derive the label from the key)* |

### 16.7 `base_config` → `account_roles`

| Legacy | Proposed |
|---|---|
| `BC_ID` | `role` (enum) |
| `BC_SSN` | `account_id` |
| `BC_Name` | *(drop — derive from `role`)* |
| `BC_Enabled` | `is_enabled` |
| `BC_Default` | `is_default` |

Role enum values: `11` → `CashChequeIssue`, `12` → `PostdatedChequeIssue`, `13` → `NotesPayable`,
`14` → `NotesReceivableOnHand`, `15` → `NotesInCollection`.

### 16.8 Forms → React routes/components

| Legacy unit / form | Purpose | Proposed component | Proposed route |
|---|---|---|---|
| `SNewu` / `TSNew` | Chart of accounts browser + CRUD | `ChartOfAccountsPage` | `/accounts` |
| `Sarfasl_TakmilU` | Supplementary account data | `AccountDetailsDialog` | `/accounts/:id/details` |
| `CodeNameU` | Code + name prompt | `CodeNameDialog` | — |
| `SelectSarfasl` | Single-level account browse | `AccountLevelPicker` | — |
| `Sarfasl_SelectU` | Leaf-account picker with filters | `AccountPicker` | — |
| `FGetCodeU` (frame) | Embedded 4-level code entry | `AccountCodeInput` | — |
| `TarafU` | 4-level code entry + validation | `AccountCodeInput` (same component) | — |
| `SanadViewU` | Voucher browser by state | `VoucherListPage` | `/vouchers?status=` |
| `SanadEditU` | Voucher editor / viewer | `VoucherEditorPage` | `/vouchers/new`, `/vouchers/:id`, `/vouchers/:id/edit` |
| `EditArticleMoeinU` | Voucher line dialog | `VoucherLineDialog` | — |
| `SanadMoeinu` (legacy) | Legacy voucher screen | *(retire)* | — |
| `ArticleMoeinu` (legacy) | Legacy line dialog | *(retire)* | — |
| `ArticleRooznamehU` | Journal line dialog | `JournalLineDialog` | — |
| `Sanad_NDU` | Voucher number + date prompt | *(fold into `VoucherEditorPage`)* | — |
| `MergeSanad` | Merge two vouchers | `MergeVouchersDialog` | — |
| `MoeinSearchU` | Voucher-line search | `VoucherLineSearchPage` | `/vouchers/lines/search` |
| `RooznamehViewU` | Journal voucher browser | `JournalListPage` | `/journal` |
| `MoeinToRU` | Journal generation | `GenerateJournalDialog` | `/journal/new` |
| `MakeRooznamehU` (legacy) | Legacy journal generation | *(retire)* | — |
| `SodoorSanadU` | Inventory posting list | `InventoryPostingPage` | `/postings/inventory` |
| `MakeSanadU` | Generated-voucher preview | `GeneratedVoucherPreview` | — |
| `NewFinalu` | Close the books | `CloseBooksPage` | `/period-close/close-accounts` |
| `EnteghalU` | Year-end carry-forward | `CarryForwardPage` | `/period-close/carry-forward` |
| `FinalU` (dead) | — | *(do not port)* | — |
| `BastanHesab` | Balance export to `.GGS` | `ExportBalancesDialog` | — |
| `TajmiU` | Consolidated subsidiary ledger | `ConsolidatedLedgerPage` | `/reports/consolidated-ledger` |
| `KolStateU` | General-ledger control list | `GeneralLedgerControlPage` | `/reports/gl-control` |
| `KolSatateU` (dead) | — | *(do not port)* | — |
| `BedBes` | Debtors and creditors | `DebtorsCreditorsPage` | `/reports/debtors-creditors` |
| `SarfaslChap` (stub) | — | *(do not port)* | — |
| `ListSarfaslu` (unreachable) | Legacy account list | *(do not port)* | — |
| `NewSarfaslu` (unreachable) | Legacy account create | *(do not port)* | — |
| `Sarfasl_Kolu` (dead) | — | *(do not port)* | — |
| `Sarfasl_ListU` (unreachable) | — | *(do not port)* | — |
| `S_KolU` (dead) | — | *(do not port)* | — |
| `Admin` | User and permission admin | `UserAdminPage` | `/admin/users` |
| `TanzimU` | Application/fiscal-year settings | `SettingsPage` | `/settings` |
| `MakeNewU` | Create a fiscal year | `NewFiscalYearDialog` | `/settings/fiscal-years/new` |

### 16.9 Data-module routines → Rust functions

| Legacy | Proposed | Module |
|---|---|---|
| `Dm.New_Sanad` | `next_voucher_number(fiscal_year_id)` | `accounting::vouchers` |
| `Dm.Get_NewSanad_DateID` | `voucher_number_for_generated(fiscal_year_id, date, source_kinds)` | `accounting::vouchers` |
| `Dm.Get_SanadMaxTX` / `Dm.Moein_Tx` | `voucher_status(voucher_id)` | `accounting::vouchers` |
| `Dm.Get_SanadFound` | `voucher_exists(...)` | `accounting::vouchers` |
| `Dm.Get_SanadDate` | `voucher_date(...)` | `accounting::vouchers` |
| `Dm.DMoein_Make` | `upsert_voucher_header(...)` | `accounting::vouchers` |
| `Dm.Dmoein_UpdateMab` | `refresh_voucher_totals(...)` | `accounting::vouchers` |
| `Dm.Delete_Sanad_moein` | `delete_voucher(...)` | `accounting::vouchers` |
| `Dm.Delete_Moein_ssn` | `delete_voucher_line(...)` | `accounting::vouchers` |
| `Dm.Sarfasl_Seek` | `find_account_by_code(...)` | `accounting::accounts` |
| `Dm.Sarfasl_SSN_CODEName` | `format_account_code_rtl(...)` | `accounting::accounts` |
| `Dm.Split_Code` | `parse_account_code(...)` | `accounting::accounts` |
| `Dm.is_Sarfasl_Last_Deep` / `_SSN` | `is_leaf_account(...)` | `accounting::accounts` |
| `Dm.Update_Sarfasl_Child` | `refresh_account_child_counts()` | `accounting::accounts` |
| `Dm.Get_LastCodeName` | `account_short_label(...)` | `accounting::accounts` |
| `Dm.Get_Jari_Code` | `party_identity_for_account(...)` | `parties` |
| `Dm.IsEnabel` | `has_permission(user_id, key)` | `platform::auth` |
| `Dm.Is_Admin_Or_Valid_Sanad` | `can_access_voucher(...)` | `platform::auth` |
| `Dm.Is_Admin_Or_Valid_Daftar` | `can_access_account(...)` | `platform::auth` |
| `Dm.Is_New_Sanad_Valid` | `require_open_fiscal_year(...)` | `accounting::periods` |
| `Dm.From_Date` / `Dm.To_Date` | `fiscal_year_range(...)` | `accounting::periods` |
| `Dm.IsDate` / `Dm.isValidDate` | `parse_jalali` / `validate_in_fiscal_year` | `platform::dates` |
| `Dm.MiladiToShamsi` | `gregorian_to_jalali(...)` | `platform::dates` |
| `Dm.inttostr3` | `format_amount(...)` | `platform::format` |
| `Dm.Str2String` / `Dm.N23` | `amount_to_persian_words(...)` | `platform::format` |
| `Dm.SanDoogh_k/_M/_KM` | `notes_on_hand_account()` | `treasury::config` |
| `Dm.Jaryan_K/_M/_KM` | `notes_in_collection_account()` | `treasury::config` |
| `Dm.Get_paramstr` / `Set_paramstr` | `get_setting` / `set_setting` | `platform::settings` |

### 16.10 Enumerations

| Legacy | Values | Proposed enum |
|---|---|---|
| `DM_TX` / `M_Tx` | 0, 1, 2 | `VoucherStatus { Draft, Approved, Posted }` |
| `DM_Kind` / `M_Kind` | 1, 2 | `VoucherKind { Subsidiary, Journal }` |
| `M_Id` | 0; 1–9; 11–19; 21–29; 31–39 | `SourceKind { Manual, Sales, Pistachio, Treasury, Inventory }` + a document sub-type |
| `M_Id` (inventory) | 31, 32, 33, 35 | `InventoryPosting { OpeningStock, Purchase, Sale, SalesReturn }` |
| `FM_Lock` | 0, 1, 2 | `DocumentPostingStatus { Unapproved, Approved, Posted }` |
| `FM_ID` | 11, 12, 13, 14, 15, 16, 22, 25, 26 | `InventoryDocumentType { OpeningStock, Purchase, SalesReturn, PurchaseReturn, ProductionReceipt, TransferIn, Sale, ProductionIssue, TransferOut }` (mapping to be confirmed — §14 Q33) |
| `BC_ID` | 11–15 | `AccountRole` (see §16.7) |
| `_NewEditView` | 1, 2, 3 | `EditorMode { New, Edit, View }` |
| `SNewu.State` | 1, 2, 3, 4 | `AccountLevel { General, Subsidiary, Analytic, SubAnalytic }` |

---

*End of specification.*

---

_Prev: [03-15-proposed-improvements-needs-user-approval](03-15-proposed-improvements-needs-user-approval.md)_
