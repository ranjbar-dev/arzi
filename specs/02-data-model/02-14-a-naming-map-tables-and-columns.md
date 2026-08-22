_Part of [02-data-model](../02-data-model.md) — [index](00-index.md)_

## 14. Naming map

Complete legacy → proposed mapping for everything named in §2 and §3, so that no identifier is lost
in translation. Naming conventions are those adopted in `docs/01-glossary.md` §7: PostgreSQL
`snake_case`, **plural** table names, **singular** column names, `*_id` for foreign keys, `is_*`
for booleans, `*_at` for timestamps, `*_date` for dates.

Legend: **drop** = the column does not exist in the target model (derived, dead, or replaced);
**→ §13.n** = the mapping depends on an unapproved proposal; **?** = mapping unverified, see §12.

---

### 14.1 Tables

| # | Legacy table | Proposed table | Domain | Notes |
|---|---|---|---|---|
| 1 | `Base` | `fiscal_years` **+** `organization` **+** `account_code_format` **+** `system_accounts` | platform | one legacy table becomes four (§8.6). Split is **→ §13.9**. |
| 2 | `Base_Config` | `system_accounts` | accounting | merged with `Base.C1081/C1082` **→ §13.10** |
| 3 | `Sarfasl` | `accounts` | accounting | global, not year-scoped |
| 4 | `Moein` | `voucher_lines` | accounting | *not* a subsidiary ledger despite the name |
| 5 | `DMoein` | `vouchers` | accounting | voucher header |
| 6 | `Sahamdar` | `parties` | parties | *not* a shareholder table (`01-glossary.md` §6b) |
| 7 | `DCheck` | `cheques` (received) | treasury | |
| 8 | `DCheck2` | `cheque_events` | treasury | append-only event log |
| 9 | `DFish` | `deposit_slips` | treasury | |
| 10 | `TCheck` | `cheque_types` **?** | treasury | confidence C; DDL required (§12.5) |
| 11 | `CheckMaster` | `cheque_payment_documents` | treasury | issued-cheque batch header |
| 12 | `CheckDetail` | `cheque_payment_lines` | treasury | |
| 13 | `TankhahMaster` | `petty_cash_documents` | treasury | |
| 14 | `TankhahDetail` | `petty_cash_lines` | treasury | |
| 15 | `Anbar_Factor` | `inventory_invoices` | inventory | |
| 16 | `Anbar_FactorD` | `inventory_invoice_lines` | inventory | |
| 17 | `Anbar_Jens` | `items` | inventory | `جنس` = goods/item |
| 18 | `Anbar_Config` | `warehouses` | inventory | one row per warehouse (§8.4) |
| 19 | `Anbar_Vahed` | `units_of_measure` | inventory | |
| 20 | `Kinds` | `product_grades` | inventory | pistachio grading |
| 21 | `Tanzim` | `app_settings` | platform | key/value, ids 1001–1015 (§8.2) |
| 22 | `PassWord` | `users` | platform | |
| 23 | `Pass_Config` | `user_permissions` | platform | |
| 24 | `TolidMaster` | *(none — placeholder)* | — | every reference is dummy SQL (§12.14) |
| 25 | `Moadian` | `tax_submissions` | compliance | `مؤدیان` = taxpayers (the Iranian tax portal) |
| 26 | `temp_*` | *(none)* | — | transient scratch tables; replaced by CTEs |
| — | *(new)* | `user_preferences` | platform | replaces the per-workstation ini file (§8.6) |
| — | *(new)* | `settings_audit_log` | platform | **→ §13.19** |
| — | *(new)* | `banks`, `bank_branches`, `bank_accounts`, `cheque_books` | treasury | **→ §13.13** |
| E1 | `Anbar.dbo.Cala` | *external* | inventory | do not port; integrate (§1.5) |
| E2 | `Anbar.dbo.Anbar` | *external* | inventory | |
| E3 | `Anbar.dbo.FactorMaster` | *external* | inventory | `FM_Factor` seeded at `1700001` (§5.3.4) |
| E4 | `Anbar.dbo.FactorDetail` | *external* | inventory | |
| E5 | `Saham.dbo.NSaham` | *external* | shares | the real share registry |
| E6 | `Rppc_Solution.dbo.NewRamz` | *external* | pistachio receipts | reached via `B_SelectSerial` |
| — | `BN_*` (`BankTanzim`) | `banks` **?** | treasury | table may not exist (§12.13) |

**Not tables** (datasets that look like tables in `TDM`): `Anbar_Tasfieh` (a parameterised query
over `Anbar_Factor`/`DCheck`/`DFish`, `Dmu.dfm:1017-1110`), `Base_Q`, `QCheck`, `QDCheck`,
`Jari_Rem`, `SahamdarConfig`, `KharidPeste_List`, `Anbar_Jens_Phi1`, `AnbarCala_SeekName`.

---

### 14.2 Column-name patterns

| Legacy pattern | Proposed pattern | Example |
|---|---|---|
| `<PFX>_SSN` | `id` | `M_SSN` → `voucher_lines.id` |
| `<PFX>_COID` / `_Coid` / `_CoID` | `fiscal_year_id` | casing is inconsistent in the legacy, even within one table's queries |
| `<PFX>_Mab` | `amount` / `total_amount` | `bigint`, whole rial (§7) |
| `<PFX>_Date` | `<thing>_date` | `date`, Gregorian; Jalali derived (§6.8) |
| `<PFX>_Desc` | `description` | |
| `<PFX>_Sanad` | `voucher_number` (or `voucher_id`) | |
| `<PFX>_UserID` / `_User` | `created_by` | but see §13.19 — legacy overwrites it on edit |
| `<PFX>_State` | `status` | typed enum, not `int` |
| `<PFX>_StateName` | **drop** | denormalised Persian label; derive (§13.11) |
| `<PFX>_LinkPrg` + `<PFX>_LinkSSN` | `source_module` + `source_id` | polymorphic; **→ §13.8** |
| `<PFX>_BedSSN` / `_BedCR` / `_BedName` | `<role>_account_id` / **drop** / **drop** | `CR`/`Name` are denormalised `Sarfasl` copies (§13.11) |
| `<PFX>_BesSSN` / `_BesCR` / `_BesName` | `<role>_account_id` / **drop** / **drop** | |
| `<PFX>_Count` | `line_count` | denormalised; drift-prone (§7.7) |
| `<PFX>_Lock` | `is_locked` | `boolean` |
| `Bed` / `Bes` | `debit` / `credit` | `بدهکار` / `بستانکار` |
| `Ted` | `quantity` | `تعداد` |
| `Phi` | `unit_price` | `فی` |
| `Kasr` | `deduction` | `کسر` |
| `Maliat` | `tax` / `tax_amount` | `مالیات` (VAT) |
| `Kol` (in inventory) | `line_gross` | `کل` = total. **Not** the same `Kol` as the account level. |
| `Vahed` | `unit_of_measure` | `واحد` |
| `Manfi` | `allow_negative_stock` | `منفی` = negative |

---

### 14.3 `Base` → `fiscal_years` / `organization` / `account_code_format` / `system_accounts`

| Legacy column | Proposed | Target table |
|---|---|---|
| `CO_ID` | `year` (and the surrogate `id`) | `fiscal_years` |
| `FromDate` | `start_date` | `fiscal_years` |
| `ToDate` | `end_date` | `fiscal_years` |
| `IsActive` | `is_active` | `fiscal_years` |
| `BackupDir` | **drop** (§10.8) | — |
| `Co_Name` | `name` | `organization` |
| `Co_Sub` | `subtitle` | `organization` |
| `Co_Address` | `address` | `organization` |
| `Co_Tel` | `phone` | `organization` |
| `Co_Fax` | `fax` | `organization` |
| `Co_Web` | `website` | `organization` |
| `Co_EMail` | `email` | `organization` |
| `Co_Sabt` | `registration_number` | `organization` |
| `Co_Melli` | `national_id` | `organization` |
| `Co_Egh` | `economic_code` | `organization` |
| `Co_Post` | `postal_code` | `organization` |
| `ARM` | `logo` | `organization` |
| `No_Ko` | `width` where `level = 1` | `account_code_format` |
| `No_Mo` | `width` where `level = 2` | `account_code_format` |
| `No_Ta1` | `width` where `level = 3` | `account_code_format` |
| `No_Ta2` | `width` where `level = 4` | `account_code_format` |
| `Real_Len` | **drop** (dead) | — |
| `C1081` | `account_id` where `role = 'cash'` | `system_accounts` |
| `C1081C` | **drop** (derive) | — |
| `C1082` | `account_id` where `role = 'cheques_in_transit'` | `system_accounts` |
| `C1082C` | **drop** (derive) | — |

### 14.4 `Base_Config` → `system_accounts`

| Legacy | Proposed |
|---|---|
| `BC_ID` | `role` (text enum; legacy integer kept in the migration mapping only) |
| `BC_SSN` | `account_id` |
| `BC_Name` | `label_fa` |
| `BC_Default` | `is_default` |
| `BC_Enabled` | `is_enabled` |

Known role ids (§2.4): `13` → `notes_payable` (`اسناد پرداختنی`), `14` → `notes_receivable`
(`اسناد دریافتنی`), `15` → `notes_in_collection` (`اسناد در جریان وصول`), `11` → **unidentified**
(§12.9).

### 14.5 `Sarfasl` → `accounts`

| Legacy | Proposed |
|---|---|
| `S_SSN` | `id` |
| `S_Ko` | `general_ledger_code` (`کل`) |
| `S_Mo` | `subsidiary_code` (`معین`) |
| `S_Ta1` | `analytic1_code` (`تفصیلی ۱`) |
| `S_Ta2` | `analytic2_code` (`تفصیلی ۲`) |
| `S_Name` | `name` |
| `S_Child` | `child_count` (or derive `is_leaf`, **→ §13.6**) |
| `S_Lock` | `is_locked` |
| `FullName` | **drop** (stale; maintenance commented out, `Dmu.pas:283-296`) |
| `M_L` | **drop** (derive; `dbo.Make_L`, §12.4) |
| `M_R` | **drop** (derive; `dbo.Make_R`) |
| `S_Address` | `address` |
| `S_Tel` | `phone` |
| `S_Fax` | `fax` |
| `S_Sabt` | `registration_number` |
| `S_Egh` | `economic_code` |
| `S_Post` | `postal_code` |
| `S_Melli` | `national_id` |
| `S_Active` | `is_active` (semantics unconfirmed, §12.10) |
| `S_IS_Check`, `S_IS_Fish`, `S_IS_APArdakhti`, `S_IS_ADaryafti` | **drop** — superseded by `system_accounts` (§2.5) |
| *(new)* | `parent_id` **→ §13.6** |
| *(new)* | `party_id` **→ §13.7** |

### 14.6 `Sahamdar` → `parties`

| Legacy | Proposed | | Legacy | Proposed |
|---|---|---|---|---|
| `S_SSN` | `id` | | `S_CodeMelli` | `national_id` |
| `S_Card` | `card_number` | | `S_CodePosti` | `postal_code` |
| `S_Kind` | `party_type` | | `S_CodeSabt` | `registration_number` |
| `S_Name` | `first_name` | | `S_Mobile` | `mobile` |
| `S_Famil` | `last_name` | | `S_Phone` | `phone` |
| `S_Father` | `father_name` | | `S_Siba` | `bank_account_siba` |
| `S_BDate` | `birth_date` | | `S_ShabaNo` | `iban` |
| `S_BPlace` | `birth_place` | | `S_MaliatState` | `tax_status` |
| `S_SDate` | `id_issue_date` | | `S_Shanas` | `entity_national_id` |
| `S_SPlace` | `id_issue_place` | | `S_Lock` | `is_locked` |
| `S_IDNO` | `id_card_number` (**`text`**, not `int` — §2.6) | | `S_Aks` / `S_AKS` | `photo` |
| `S_Address` | `address` | | | |

### 14.7 `Moein` → `voucher_lines`

| Legacy | Proposed | | Legacy | Proposed |
|---|---|---|---|---|
| `M_SSN` | `id` | | `M_Code` | `account_id` |
| `M_COID` | `fiscal_year_id` | | `M_Tx` | `status` |
| `M_Sanad` | `voucher_number` (→ `voucher_id`) | | `M_Kind` | `journal_kind` |
| `M_Date` | `line_date` | | `M_ID` | `source_module` |
| `M_Bed` | `debit_amount` | | `M_Link` | `source_id` |
| `M_Bes` | `credit_amount` | | `M_User` | `created_by` |
| `M_Ted` | `quantity` | | `M_Time` | `created_at` |
| `Article` / `M_Article` | `description` (**two names, §12.10**) | | `M_L`, `M_R`, `M_Name`, `M_CR`, `M_CodeStr` | **drop** (derive) |
| `M_Ko`, `M_Mo`, `M_Ta1`, `M_Ta2` | **drop** (denormalised from `accounts`; keep only if `account_id` cannot be resolved for every row — §12.11) | | | |

`M_ID` source-module codes → proposed enum (`journal_source`):

| `M_ID` | Proposed variant | Persian / meaning |
|---|---|---|
| 1–9 | `inventory_invoice` (subtypes 1–9) | `فاکتور انبار` |
| 15 | **unidentified** (§12.9) | — |
| 21 | `cheque_received` | `چک دریافتی` |
| 22 | `cheque_bounced` | `برگشت چک از بانک` |
| 23 | `cheque_collected` | `وصول چک` |
| 24 | `cheque_returned_to_issuer` | `استرداد چک` |
| 25 | `deposit_slip` | `فیش` |
| 26 | `cheque_payment_document` | `CheckMaster` |
| 27–29 | further treasury events (unlabelled) | — |
| 34 | `pistachio_purchase_receipt` | `خرید پسته` |
| 35 | **unidentified** (§12.9) | — |
| 41 | `petty_cash` | `تنخواه` |

### 14.8 `DMoein` → `vouchers`

| Legacy | Proposed | | Legacy | Proposed |
|---|---|---|---|---|
| `DM_SSN` | `id` | | `DM_Kind` | `journal_kind` |
| `DM_Coid` | `fiscal_year_id` | | `DM_Lock` | `is_locked` |
| `DM_Sanad` | `voucher_number` | | `DM_Atf` | `cross_reference` **?** (§12.10) |
| `DM_Date` | `voucher_date` | | `DM_CUser` | `updated_by` **?** — C/M are **swapped** (§2.8) |
| `DM_Desc` | `description` | | `DM_CDate` | `updated_at` **?** |
| `DM_TBed` | `total_debit` | | `DM_MUser` | `created_by` **?** |
| `DM_TBes` | `total_credit` | | `DM_MDate` | `created_at` **?** |
| `DM_Count` | `line_count` | | | |
| `DM_Tx` | `status` | | | |

`M_Tx` / `DM_Tx` → `voucher_status`: `0` → `draft` (`موقت`), `1` → `confirmed` (`تأیید شده`),
`2` → `posted` (`ثبت قطعی`). `M_Kind` / `DM_Kind` → `journal_kind`: `1` → `ledger` (`معین`),
`2` → `daybook` (`روزنامه`).


---

[← 02-13-b-improvements-security-and-audit.md](02-13-b-improvements-security-and-audit.md) | [02-14-b-naming-map-procedures-and-modules.md →](02-14-b-naming-map-procedures-and-modules.md)
