# 01 — Glossary: Persian / Finglish → English

The legacy codebase names everything in transliterated Persian ("Finglish"). This file is the
authoritative translation table used by every other document in `docs/`. When a specification
document proposes an English name for a table, column, screen or concept, it comes from here.

Read this first. Nothing else in `docs/` is intelligible without it.

---

## 1. Accounting

| Legacy term | Persian | Meaning | Proposed English term |
|---|---|---|---|
| Hesab | حساب | Account | `account` |
| Sarfasl | سرفصل | Chart-of-accounts head / account definition | `account` / `chart_of_accounts` |
| Kol | کل | General ledger level (top account level) | `general_ledger` / level 1 |
| Moein | معین | Subsidiary account level (second level) | `subsidiary` / level 2 |
| Tafsil / Tafzili | تفصیلی | Analytic / detail level (third level) | `analytic` / level 3 |
| Sanad | سند | Accounting voucher / journal document | `voucher` |
| Article / Artikel | آرتیکل | One line of a voucher | `voucher_line` |
| Bed / Bedehkar | بدهکار | Debit | `debit` |
| Bes / Bestankar | بستانکار | Credit | `credit` |
| Mandeh | مانده | Balance (remaining) | `balance` |
| Gardesh | گردش | Turnover / movement in a period | `turnover` |
| Rooznameh | روزنامه | Journal / daybook | `journal` |
| Daftar | دفتر | Ledger book | `ledger` |
| Daftar Kol | دفتر کل | General ledger | `general_ledger` |
| Daftar Moein | دفتر معین | Subsidiary ledger | `subsidiary_ledger` |
| Taraz | تراز | Trial balance | `trial_balance` |
| Setooni / Sotoon | ستونی / ستون | Column-based / column | `column` |
| Taraz 4 Setooni | تراز ۴ ستونی | 4-column trial balance | `trial_balance_4col` |
| Taraz 6 Setooni | تراز ۶ ستونی | 6-column trial balance | `trial_balance_6col` |
| Bastan Hesab | بستن حساب | Closing the books | `period_close` / `year_end_close` |
| Enteghal | انتقال | Transfer / carry-forward to next period | `carry_forward` |
| Tajmi / Tajmi'e | تجمیع | Aggregation / consolidation | `consolidation` |
| Sodoor | صدور | Issuing (a voucher) | `issue` / `post` |
| Sal Mali | سال مالی | Fiscal year | `fiscal_year` |
| Dore | دوره | Period | `period` |
| Sharh | شرح | Description / narration | `description` |
| Mablagh / Mab | مبلغ | Amount | `amount` |
| Tashkhis | تشخیص | Debit/credit side indicator | `side` |
| Moavaghe | معوقه | Overdue / outstanding | `overdue` |
| Sood | سود | Profit | `profit` |
| Zian | زیان | Loss | `loss` |
| Sarmaye | سرمایه | Capital | `capital` |
| Daraei | دارایی | Asset | `asset` |
| Bedehi | بدهی | Liability | `liability` |
| Hazineh | هزینه | Expense | `expense` |
| Daramad | درآمد | Revenue / income | `revenue` |

## 2. Inventory / warehouse

| Legacy term | Persian | Meaning | Proposed English term |
|---|---|---|---|
| Anbar | انبار | Warehouse / stock | `warehouse` / `inventory` |
| Cala / Kala | کالا | Goods / item | `item` |
| Jens | جنس | Article / commodity | `item` |
| Ajnas | اجناس | Goods (plural) | `items` |
| Card Jensi | کارت جنسی | Stock card / item movement card | `item_ledger_card` |
| Mandeh Anbar | مانده انبار | Stock on hand | `stock_on_hand` |
| Factor | فاکتور | Invoice / bill | `invoice` |
| Kharid | خرید | Purchase | `purchase` |
| Foroosh | فروش | Sale | `sale` |
| Vorood | ورود | Inbound / goods receipt | `receipt` / `inbound` |
| Khorooj | خروج | Outbound / goods issue | `issue` / `outbound` |
| Amalkard | عملکرد | Performance / activity report | `activity` |
| Tasfieh | تسویه | Settlement / clearing | `settlement` |
| Tedad | تعداد | Quantity / count | `quantity` |
| Vazn | وزن | Weight | `weight` |
| Gheymat | قیمت | Price | `price` |
| Vahed | واحد | Unit of measure | `unit_of_measure` |
| Takhfif | تخفیف | Discount | `discount` |
| Pesteh | پسته | Pistachio (product-specific module) | `pistachio` (domain module) |
| Serial | سریال | Serial / lot number | `serial` / `lot` |
| Bastebandi | بسته‌بندی | Packaging | `packaging` |

## 3. Treasury

| Legacy term | Persian | Meaning | Proposed English term |
|---|---|---|---|
| Check | چک | Cheque | `cheque` |
| Daryaft | دریافت | Receipt / received | `receipt` / `received` |
| Pardakht | پرداخت | Payment / paid | `payment` / `paid` |
| Vosool | وصول | Collected / cleared | `cleared` |
| Bargasht | برگشت | Bounced / returned unpaid | `bounced` |
| Esterdad | استرداد | Returned to the issuer | `returned` |
| Khab / Dar Jarian | خواب / در جریان | In hand / in transit | `on_hand` / `in_transit` |
| Sarresid | سررسید | Due date | `due_date` |
| Fish | فیش | Bank deposit slip | `deposit_slip` |
| Ghabz | قبض | Receipt document | `receipt_note` |
| Tankhah | تنخواه | Petty cash / imprest fund | `petty_cash` |
| Bank | بانک | Bank | `bank` |
| Shobe | شعبه | Branch | `branch` |
| Asnad Daryaftani | اسناد دریافتنی | Notes receivable | `notes_receivable` |
| Asnad Pardakhtani | اسناد پرداختنی | Notes payable | `notes_payable` |
| Sandogh | صندوق | Cash box / till | `cash_account` |

## 4. Parties and organisation

| Legacy term | Persian | Meaning | Proposed English term |
|---|---|---|---|
| Taraf / Taraf Hesab | طرف حساب | Counterparty (customer or supplier). **Note: the `TarafU` unit is an account-code picker, not a party table — see §6b.** | `party` / `counterparty` |
| Moshtari | مشتری | Customer | `customer` |
| Forooshande | فروشنده | Supplier / vendor | `supplier` |
| Sahamdar | سهامدار | Shareholder | `shareholder` |
| Saham | سهام | Shares / equity | `shares` |
| Sherkat | شرکت | Company | `company` |
| Jari | جاری | Current (as in "current account") | `current_account` |
| Karbar | کاربر | User | `user` |
| Modir | مدیر | Manager / administrator | `admin` |

## 5. Application / UI

| Legacy term | Persian | Meaning | Proposed English term |
|---|---|---|---|
| Tanzim / Tanzimat | تنظیم / تنظیمات | Setting / settings | `settings` |
| Chap | چاپ | Print | `print` |
| Royat | رویت | View / inspect | `view` |
| Jostojoo | جستجو | Search | `search` |
| Zakhire | ذخیره | Save | `save` |
| Hazf | حذف | Delete | `delete` |
| Virayesh | ویرایش | Edit | `edit` |
| Jadid | جدید | New | `new` |
| Taeed | تایید | Confirm / approve | `confirm` |
| Enseraf | انصراف | Cancel | `cancel` |
| Gozaresh | گزارش | Report | `report` |
| Taghirat | تغییرات | Changes (audit log) | `audit_log` |
| Poshtiban / Backup | پشتیبان | Backup | `backup` |
| Ramz | رمز | Password | `password` |
| Dastresi | دسترسی | Access / permission | `permission` |
| Sath | سطح | Level | `level` |
| Radif | ردیف | Row / line number | `line_number` |
| Codeh | کد | Code | `code` |
| Nam | نام | Name | `name` |
| Tarikh | تاریخ | Date | `date` |
| Shomare | شماره | Number | `number` |

## 6. Recurring column abbreviations in the legacy schema

These appear as literal column names across the SQL Server tables. The rebuild renames them;
this table is what keeps the mapping recoverable.

| Legacy column | Meaning | Proposed column name |
|---|---|---|
| `SSN` | Surrogate primary key (identity), *not* a social security number | `id` |
| `COID` / `CO_ID` | **Fiscal year identifier — NOT a company/tenant id.** See note below. | `fiscal_year_id` |
| `CR` | Account code (the "code-e hesab") | `account_code` |
| `Mab` | Amount (`Mablagh`) | `amount` |
| `Desc` | Description | `description` |
| `Date` | Jalali date stored as a string | `date_jalali` (+ derived `date`) |
| `DateS` | Secondary/system date, usually the due date or entry date | see per-table docs |
| `State` | Numeric status code | `status` |
| `StateName` | Denormalised status label (Persian) | dropped; derived from `status` |
| `UserID` | Creating/modifying user | `created_by` / `updated_by` |
| `linkPrg` | Source module identifier of a linked document | `source_module` |
| `LinkSSN` | Primary key of the linked source document | `source_id` |
| `BedSSN` / `BedCR` / `BedName` | Debit-side account id / code / denormalised name | `debit_account_id` / `debit_account_code` |
| `BesSSN` / `BesCR` / `BesName` | Credit-side account id / code / denormalised name | `credit_account_id` / `credit_account_code` |
| `Zssn` / `ZCR` / `ZName` | Third-party ("Zi-nafa") account id / code / name | `third_party_account_id` etc. |
| `Sanad` | Voucher number the record was posted under | `voucher_id` |
| `SN` | Serial / document number | `document_number` |

## 6b. Corrections to the obvious reading — traps in the legacy naming

Four legacy names mean something other than what they look like. Every one of these was wrong in
the first draft of this glossary and was corrected only after reading the code. Treat the legacy
identifiers as unreliable labels.

| Legacy name | Looks like | Actually is | Evidence |
|---|---|---|---|
| `CO_ID` / `COID` | Company id (tenant) | **Fiscal year id.** `Base` holds one row per fiscal year, and that row also carries the operating entity's letterhead identity. One physical database; years separated by a `*_COID` stamp on every transactional table. Master data (`Sarfasl`, `Sahamdar`) has no year column and is global. "Multi-company" is emulated by adding `Base` rows with a different name — there is no isolation. | `Dmu.pas:113,745-749`; `TanzimU.pas:121-143` |
| `Sahamdar` ("shareholder") | Shareholder / equity register | **Person and legal-entity register.** `S_Kind=1` natural person, `S_Kind=2` legal entity. No share count, nominal value, percentage, join/exit date or profit-allocation logic exists anywhere in this codebase. The real share registry lives in a separate database (`Saham.Dbo`, `\\pesteh\SahamData\`), feature-detected at runtime and only read for display. | `Dmu.pas:757-780`; `CardJariU.pas:304-337` |
| `Taraf` ("counterparty") | Customer/supplier master table | **A 4-segment account-code picker widget** over `Sarfasl` (Kol / Moein / Tafsil1 / Tafsil2). No CRUD, no table of its own. A counterparty *is* a leaf `Sarfasl` node (`S_Child = 0`); its address, phone, national ID and tax IDs live on the account row (`S_Address`, `S_Tel`, `S_Melli`, `S_Egh`, `S_Sabt`, `S_Post`). | `TarafU.pas:104-536`; `Sarfasl_TakmilU.pas:65-84` |
| `Kind_Table` | Account types / natures | **Pistachio product grades.** Belongs to the inventory domain, not accounting. There is no account-type column at all — account nature is implied by hard-coded Kol number ranges. | `Sarfasl_SelectU.pas` |
| `ChangesU` ("Taghirat" = changes) | Audit / change log | **Fiscal-year selector.** No audit trail exists anywhere in the system. | `ChangesU.pas` |
| `CS2` (in `arzi.local.ini`) | Licence key | **The SQL Server connection string including credentials**, obfuscated with the constant `53269` compiled into the binary. `CS3` is the licence; `CS1` a flag; `CS31` dead. | `Dmu.pas:726-737`; `INI.pas:43-170` |
| `test.dpr` / `testmainU` | Test suite | **Licence-key generator.** The project has no automated tests of any kind. | `testmainU.pas` |

## 7. Naming conventions adopted for the rebuild

Decided once here, applied everywhere in `docs/`:

- **PostgreSQL**: `snake_case`, plural table names (`vouchers`, `voucher_lines`), singular column
  names, `id` as the primary key, `<table_singular>_id` for foreign keys, `created_at` /
  `updated_at` / `created_by` / `updated_by` for audit columns, `_at` suffix for timestamps.
- **Rust**: `snake_case` functions and fields, `PascalCase` types, module names matching the
  domain (`accounting`, `inventory`, `treasury`, `parties`, `platform`).
- **React/TypeScript**: `PascalCase` components, `camelCase` props and variables. API payloads use
  `camelCase`; the Rust layer serialises with `#[serde(rename_all = "camelCase")]`.
- **API routes**: `/api/v1/<plural-resource>`, kebab-case for multi-word resources
  (`/api/v1/voucher-lines`).
- **Persian remains in the UI, English in the code.** Every user-visible string is a translation
  key; the Persian text from the legacy forms is the initial `fa` locale value. No Persian in
  identifiers, ever.
