_Part of [07-parties-and-shareholders](../07-parties-and-shareholders.md) — [index](00-index.md)_

## 11. Naming map

### 11.1 Tables

| Legacy | Proposed English | Notes |
|---|---|---|
| `Base` | `fiscal_period` | Split out the entity identity — see §13-I2 |
| `Sahamdar` | `party` | *Not* "shareholder" |
| `SahamdarConfig` | `party_control_account_config` | |
| `SahamdarInfo` | `party_bank_account` | |
| `Sarfasl` | `account` (chart of accounts) | |
| `Moein` | `journal_line` | (other agent's domain) |
| `DMoein` | `journal_voucher` | (other agent's domain) |
| `Kinds` | *(drop — unused)* | |
| `Saham.Dbo.NSaham` | `external_share_register.shareholder` | Read-only integration |

### 11.2 `Base` columns

| Legacy | Proposed |
|---|---|
| `Co_ID` | `fiscal_period_id` |
| `Co_Name` | `organization_name` |
| `Co_Sub` | `period_label` |
| `Co_Address` | `organization_address` |
| `Co_Tel` | `organization_phone` |
| `Co_Fax` | `organization_fax` |
| `Co_Web` | `organization_website` |
| `Co_EMail` | `organization_email` |
| `Co_Sabt` | `registration_number` |
| `Co_Melli` | `national_legal_id` |
| `Co_Egh` | `economic_code` |
| `Co_Post` | `postal_code` |
| `ARM` | `letterhead_logo` |
| `FromDate` | `starts_on` |
| `ToDate` | `ends_on` |
| `IsActive` | `is_open` |
| `BackupDir` | `backup_path` *(drop — see §13-I3)* |
| `No_Ko` / `No_Mo` / `No_Ta1` / `No_Ta2` | `level1_code_width` … `level4_code_width` |
| `Int_Len` / `Real_Len` | `integer_display_width` / `decimal_display_width` |
| `C1081` / `C1081C` | `cash_account_id` / `cash_account_code` |
| `C1082` / `C1082C` | `in_transit_account_id` / `in_transit_account_code` |
| `Kh1_Code`…`Kh8_Code` | `quick_account_ids[8]` |
| `Kh1_Desc`…`Kh8_Desc` | `quick_account_labels[8]` |

### 11.3 `Sahamdar` columns

| Legacy | Proposed |
|---|---|
| `S_Card` | `card_number` (natural business key) |
| `S_Kind` | `party_type` (`'natural_person'` \| `'legal_entity'`) |
| `S_Name` | `first_name` / `entity_name` |
| `S_Famil` | `last_name` / `representative_name` |
| `S_Father` | `father_name` |
| `S_IDNO` | `identity_document_number` |
| `S_BDate` | `birth_date` / `incorporation_date` |
| `S_BPlace` | `birth_place` / `incorporation_place` |
| `S_SDate` | `id_issue_date` |
| `S_SPlace` | `id_issue_place` |
| `S_CodeMelli` | `national_id` |
| `S_CodePosti` | `postal_code` |
| `S_CodeSabt` | `registration_code` |
| `S_Address` | `address` |
| `S_Mobile` | `mobile` |
| `S_Phone` | `phone` |
| `S_Siba` | *(drop — legacy SIBA account)* |
| `S_Shanas` | *(drop — dead)* |
| `S_MaliatState` | `tax_status` (enum, §4.2) |
| `S_Lock` | `is_locked` |

### 11.4 `Sarfasl` columns

| Legacy | Proposed |
|---|---|
| `S_SSN` | `id` |
| `S_Ko` | `level1_code` (general ledger) |
| `S_Mo` | `level2_code` (subsidiary) |
| `S_Ta1` | `level3_code` (analytic 1) |
| `S_Ta2` | `level4_code` (analytic 2) |
| `S_Name` | `name` |
| `FullName` | `full_path` |
| `M_L` / `M_R` | `tree_left` / `tree_right` |
| `S_Child` | `child_count` (`0` ⇒ `is_postable`) |
| `S_Count` | `entry_count` |
| `S_Bed` / `S_Bes` / `S_Remi` | `cached_debit` / `cached_credit` / `cached_balance` |
| `S_Active` / `S_A` | `is_active` |
| `S_Lock` | `is_locked` |
| `S_Kind` | `account_type` *(unused — confirm before porting)* |
| `S_Card` | `party_card_number` (FK → `party.card_number`) |
| `S_Address` | `party_address` |
| `S_Tel` / `S_Fax` | `party_phone` / `party_fax` |
| `S_Sabt` | `party_registration_number` |
| `S_Melli` | `party_national_id` |
| `S_Egh` | `party_economic_code` |
| `S_Post` | `party_postal_code` |
| `S_IS_Check` | `allows_cheque` |
| `S_IS_Fish` | `allows_deposit_slip` |
| `S_IS_APArdakhti` | `allows_notes_payable` |
| `S_IS_ADaryafti` | `allows_notes_receivable` |
| `NeedUpdate` | *(drop — housekeeping)* |

### 11.5 `SahamdarConfig` / `SahamdarInfo` columns

| Legacy | Proposed |
|---|---|
| `SC_K` | `level1_code` |
| `SC_M` | `level2_code` |
| `SC_T` | `fixed_level3_code` (`0` ⇒ card occupies level 3) |
| `SC_Name` | `display_name` |
| `SC_1` | `applies_to_natural_person` |
| `SC_2` | `applies_to_legal_entity` |
| `SC_Add` | `offered_by_default` |
| `SC_Rem` | `included_in_balance` |
| `SC_Kind` | `party_type` *(redundant with SC_1/SC_2 — consolidate)* |
| `SC_Tik` | *(drop — see §13-I1)* |
| `SI_SSN` | `id` |
| `SI_Card` | `party_card_number` |
| `SI_ID` | `record_type` |
| `SI_St1` | `account_identifier` (card / IBAN / account no.) |
| `SI_St2` | `account_holder` |
| `SI_St3` | `bank_name` |
| `SI_St4` | `notes` |

### 11.6 Forms and units

| Legacy unit / class | Proposed React route / component |
|---|---|
| `SahamdarU` / `TSahamdar` | `/parties` — `PartyRegister` (tabs: persons \| companies; modes: manage \| select) |
| `SahamdarEditU` / `TSahamdarEdit` | `/parties/persons/:card` — `PersonEditor` |
| `CompanyEditU` / `TCompanyEdit` | `/parties/companies/:card` — `LegalEntityEditor` |
| `SahamdarInfoU` / `TSahamdarInfo` | `/parties/:card/bank-accounts` — `PartyBankAccounts` |
| `SahamdarP` / `TSahamdarP_F` | *(drop — dead)* |
| `TarafU` / `TTaraf` | `<AccountPicker>` component |
| `SelectSarfasl` / `TSelect_Sarfasl` | `<AccountLevelList>` sub-component |
| `Sarfasl_TakmilU` / `TSarfasl_Takmil` | `AccountPartyDetailsPanel` |
| `ChangesU` / `TChangeS_F` | `<FiscalPeriodSwitcher>` (app header) |
| `TanzimU` / `TTanzimF` | `/settings/fiscal-period` — `FiscalPeriodSettings` |
| `MakeNewU` / `TMakeNew` | `/settings/fiscal-period/new` — `NewFiscalPeriod` |
| `EnteghalU` / `TEnteghalF` | `/settings/fiscal-period/rollover` — `YearEndRollover` |
| `CardJariU` / `TCardJariF` | `/parties/:card/current-account` — party-identity header only |

### 11.7 Domain vocabulary

| Persian / legacy | English |
|---|---|
| `طرف حساب` (Taraf) | counterparty |
| `سهامدار` (Sahamdar) | *(here)* party / person — **not** shareholder |
| `اشخاص` (Ashkhas) | natural persons |
| `شرکتها` (Sherkatha) | companies / legal entities |
| `حقیقی` (Haghighi) | natural (person) |
| `حقوقی` (Hoghooghi) | legal (entity) |
| `جاری` (Jari) | current account |
| `مانده` (Mandeh) | balance |
| `گردش` (Gardesh) | turnover |
| `بدهکار` (Bedehkar) / `بد` (`Bed`) | debtor / debit |
| `بستانکار` (Bestankar) / `بس` (`Bes`) | creditor / credit |
| `کل` (Kol) | general-ledger (level 1) account |
| `معین` (Moein) | subsidiary (level 2) account |
| `تفصیل` (Tafsil) | analytic / detail (levels 3-4) account |
| `سرفصل` (Sarfasl) | chart-of-accounts node |
| `سند` (Sanad) | voucher |
| `سال مالی` (Sal Mali) | fiscal year |
| `اختتامیه` (Ekhtetamieh) | closing entry |
| `افتتاحیه` (Eftetahieh) | opening entry |
| `انتقال` (Enteghal) | carry forward |
| `کد ملی` (Code Melli) | national ID (person) |
| `شناسه ملی` (Shenase Melli) | national ID (legal entity) |
| `کد اقتصادی` (Code Eghtesadi) | economic (tax) code |
| `شماره ثبت` (Shomare Sabt) | commercial registration number |
| `شبا` (Shaba) | IBAN |
| `وضعیت مالیاتی` (Vaziat Maliati) | tax status |

---


---

[← Previous](07-10-screen-by-screen-ui-specification.md) · [Index](00-index.md) · [Next →](07-12-open-questions.md)
