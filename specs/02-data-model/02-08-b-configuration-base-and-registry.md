_Part of [02-data-model](../02-data-model.md) — [index](00-index.md)_

### 8.3 Store 3 — the `Base` table (per fiscal year)

`Base` is simultaneously the fiscal-year table and the organisation record (§1.4). Edited by
`TanzimU.pas` (`TanzimF`, "settings"), created by `MakeNewU.pas` (§10).

| Column | Type (inferred) | Meaning | Edited at | Proposed |
|---|---|---|---|---|
| `CO_ID` | `int` | **fiscal year identifier** (e.g. 1403) — the logical PK | `MakeNewU.pas:119` (create only) | `fiscal_years.year` |
| `FromDate` | `char(10)` Jalali | fiscal period start; enforced by `isValidDate` (§6.4) | `TanzimU.pas:250-257` | `fiscal_years.start_date date` |
| `ToDate` | `char(10)` Jalali | fiscal period end | `TanzimU.pas:259-269` | `fiscal_years.end_date date` |
| `IsActive` | `int` | `1` = open; anything else = **archived**, blocks all posting (`Dmu.pas:1008-1014`) | not editable from `TanzimU` — see §12 | `fiscal_years.is_active boolean` |
| `Co_Name` | `varchar(100)` | organisation name (letterhead) | `TanzimU.pas:160-169` | `organization.name` |
| `Co_Sub` | `varchar(100)` | system/subtitle name (`نام سيستم`), used in the year-picker label | `TanzimU.pas:182-191`, `Dmu.dfm:362-374` | `organization.subtitle` |
| `Co_Address` | `varchar(100)` | address | `TanzimU.pas:171-180` | `organization.address` |
| `Co_Tel` | `varchar(20)` | telephone | `TanzimU.pas:204-213` | `organization.phone` |
| `Co_Fax` | `varchar(20)` | fax | `TanzimU.pas:193-202` | `organization.fax` |
| `Co_Web` | `varchar(30)` | web site | `TanzimU.pas:215-224` | `organization.website` |
| `Co_EMail` | `varchar(30)` | e-mail | `TanzimU.pas:236-245` | `organization.email` |
| `Co_Sabt` | `varchar` | company **registration number** (`شماره ثبت`) | `TanzimU.pas:137` (read) | `organization.registration_number` |
| `Co_Melli` | `varchar` | **national ID** of the legal entity (`شناسه ملی`) | `TanzimU.pas:138` | `organization.national_id` |
| `Co_Egh` | `varchar(20)` | **economic code** (`کد اقتصادی`) | `TanzimU.pas:139`, DDL at `Dmu.pas:255` | `organization.economic_code` |
| `Co_Post` | `varchar(20)` | postal code | `TanzimU.pas:140`, `Dmu.pas:256` | `organization.postal_code` |
| `ARM` | `image` | organisation **logo** (`آرم`), loaded from a file picker | `TanzimU.pas:226-234`, DDL at `Dmu.pas:260` | `organization.logo bytea` |
| `No_Ko` | `int` | **display width** (digits) of the general-ledger code segment | `TanzimU.pas:131`; applied at `Dmu.pas:1200-1202` | `account_code_widths.general_ledger` |
| `No_Mo` | `int` | display width of the subsidiary segment | `TanzimU.pas:132`; `Dmu.pas:1206-1208` | `.subsidiary` |
| `No_Ta1` | `int` | display width of analytic level 1 | `TanzimU.pas:133`; `Dmu.pas:1213-1215` | `.analytic1` |
| `No_Ta2` | `int` | display width of analytic level 2 | `TanzimU.pas:134`; `Dmu.pas:1220-1222` | `.analytic2` |
| `Real_Len` | `int` | **disabled** — the only reference is commented out | `TanzimU.pas:135` | drop |
| `BackupDir` | `varchar(100)` | destination directory for the backup routine (§10) | `TanzimU.pas:271-280` | `backup_settings.directory` |
| `C1081` | `int` → `Sarfasl.S_SSN` | **the cash account** (`صندوق`, *sandogh*). Resolved to its Kol/Moein numbers by `TDM.SanDoogh_k` / `SanDoogh_M` (`Dmu.pas:1065-1098`) | `TanzimU.pas:285` | `settings.cash_account_id` |
| `C1081C` | `varchar` | the cash account's **display code**, denormalised. Returned by `TDM.Sandoogh_KM` (`Dmu.pas:1078-1086`) | `TanzimU.pas:287-288` | drop (derive) |
| `C1082` | `int` → `Sarfasl.S_SSN` | **the cheques-in-transit / "current" account** (`جریان`, *jaryan*). `TDM.Jaryan_K` / `Jaryan_M` (`Dmu.pas:1100-1134`) | `TanzimU.pas:286` | `settings.in_transit_account_id` |
| `C1082C` | `varchar` | that account's display code, denormalised (`TDM.Jaryan_KM`, `Dmu.pas:1113-1121`) | `TanzimU.pas:289-290` | drop (derive) |

**`C1081` / `C1082` are the only settings in the whole system that change accounting behaviour** —
they name the two system accounts that treasury postings target. The opaque names are almost
certainly historical permission-key numbers reused as column names. Rename them explicitly.

Every `TanzimU` editor is the same three-line pattern — `Dm.Base.Edit; …AsString := St; Dm.Base.Post;`
— i.e. an immediate, un-transacted, un-audited write of one column, with no validation except the
`Dm.IsDate` guard on the two date fields (`TanzimU.pas:252`, `:264`).

### 8.4 Store 4 — `Anbar_Config` (per warehouse)

`TADOTable` on `Anbar_Config` (`Dmu.dfm:382-388`), edited by `AnbarTanzimU.pas`. One row per
warehouse; the grid `G1` scrolls between rows (`AnbarTanzimU.pas:196-199` re-reads on scroll).

| Column | Type | Meaning | Proposed |
|---|---|---|---|
| *(warehouse key — not visible in this unit)* | `int` | see §2 / §12 | `warehouses.id` |
| `AC_Name` | `varchar` | warehouse name | `warehouses.name` |
| `AC_DMaliat` | numeric read `AsFloat` / written `AsString` | **default VAT percentage** applied to invoice lines; forced to 0 for non-taxable items (`AnbarFactorAddU.pas:145-146`, §7.4). Mandatory — save is refused if blank (`AnbarTanzimU.pas:169-174`) | `warehouses.default_tax_rate numeric(5,2)` |
| `AC_Kharid` | `int` → `Sarfasl.S_SSN` | default account for **purchases** (`خرید`) | `purchase_account_id` |
| `AC_BKharid` | `int` → `Sarfasl.S_SSN` | default account for **purchase returns** (`برگشت خرید`) | `purchase_return_account_id` |
| `AC_Foroosh` | `int` → `Sarfasl.S_SSN` | default account for **sales** (`فروش`) | `sales_account_id` |
| `AC_BForoosh` | `int` → `Sarfasl.S_SSN` | default account for **sales returns** (`برگشت فروش`) | `sales_return_account_id` |
| `AC_Kasr` | `int` → `Sarfasl.S_SSN` | default account for **deductions/discounts** (`کسر`) | `deduction_account_id` |
| `AC_Maliat` | `int` → `Sarfasl.S_SSN` | default account for **VAT** (`مالیات`) | `tax_account_id` |

All six account columns are `Sarfasl.S_SSN` values, picked through the `Taraf` account-code widget
(`AnbarTanzimU.pas:83-99`, `114-121`) — confirming `docs/01-glossary.md` §6b that `Taraf` is a
picker, not a table. `AC_DMaliat` is written from a `TEdit` **as a string** (`AnbarTanzimU.pas:186`)
with no numeric validation beyond "not blank".

`BankTanzim.pas` is a **read-only viewer** of bank↔account mappings (`S1_Ko/S1_Mo/S1_Ta1` and
`S2_Ko/S2_Mo/S2_Ta1` composed into display codes at `BankTanzim.pas:69-88`); it persists nothing
but its own window geometry. The underlying table is identified in §2.

### 8.5 Store 5 — the Windows registry

| Path | Access | Used for |
|---|---|---|
| `HKLM\Software\<PrgName>\<KeyName>` | `TDm.GetReg_String` / `SetReg_String` (`Dmu.pas:545-565`) and `TSysInfo.GetReg_String` / `SetReg_String` (`LockUnit.pas:39-59`) | intended licence storage. The `TDm` pair is **never called** (grep: only the declarations at `Dmu.pas:126-127` and the bodies). |
| `HKLM\HARDWARE\DESCRIPTION\System` → `SystemBiosDate`, `VideoBiosDate` | read (`Dmu.pas:458-465`; `LockUnit.pas:100-120`) | machine-fingerprint inputs for the licence hash (§9.1) |
| `HKLM\HARDWARE\DESCRIPTION\System\Bios` → `SystemProductName` | read (`LockUnit.pas:122-131`) | fingerprint input |

Note both registry helpers open `HKEY_LOCAL_MACHINE` **for write** (`OpenKey(..., True)`), which
requires administrator rights on any Windows since Vista. No settings actually depend on it.

### 8.6 Proposed model

| Legacy store | Replacement |
|---|---|
| `[Base] CS2` (connection string) | environment variable `DATABASE_URL` (Docker secret / `.env`), never a file the app writes. Removes the obfuscation scheme entirely. |
| `[Base] CS1` | gone — a connection is configured or the container fails to start. |
| `[Base] CS3` / licence | **dropped**; see §13 and `docs/08-platform-and-security.md`. |
| `[Base] GridFontSize`, `[Form] Left/Top/Width/Height`, `G1FontSize`, `F0..F7`, `C1..C4`, `A4orA5`, `L1..L3`, `M_RL` | **client-side**, per user: a `user_preferences` table keyed `(user_id, scope, key)` with a `jsonb` value, exposed as `/api/v1/me/preferences`. Window geometry itself is meaningless in a browser and simply disappears; grid column widths and sort modes survive. |
| `[GetPass] ID` / `COID` | `user_preferences` (`last_fiscal_year_id`); the user id comes from the session, not a file. |
| `[AnbarReport_F] D1/D2` | `user_preferences` — but flagged: this is a *default filter*, and should be re-derived from the fiscal year rather than persisted (see §13). |
| `Tanzim` (1001–1015) | `app_settings (key text PRIMARY KEY, value text NOT NULL, label_fa text, value_type text NOT NULL)` with the fifteen rows **seeded by migration**, not lazily created on read. Typed accessors in Rust (`bool` for 1008–1010) instead of `'0'`/`'1'` strings. `Set` performs an `INSERT … ON CONFLICT DO UPDATE`, eliminating the silent no-op. Give each a readable key (`invoice.signature_1`, `invoice.show_tax`, `voucher.signature_3`, `invoice.official_footer`) and keep the numeric id only in the migration's mapping comment. |
| `Base` — year columns | `fiscal_years (id, year, start_date, end_date, is_active, created_at)` |
| `Base` — letterhead columns | single-row `organization (id CHECK (id = 1), name, subtitle, address, phone, fax, website, email, registration_number, national_id, economic_code, postal_code, logo bytea)`. **Decision needed** (§13): the legacy schema lets the letterhead differ per year because it lives on the year row. |
| `Base` — `No_Ko/No_Mo/No_Ta1/No_Ta2` | `account_code_format (level smallint PRIMARY KEY, width smallint NOT NULL CHECK (width BETWEEN 1 AND 9))` — global, since `Sarfasl` is global (§1.4). Today they are per-year, which is incoherent with a global chart of accounts. |
| `Base.C1081` / `C1082` | `app_settings` rows `accounting.cash_account_id` and `accounting.in_transit_account_id`, typed `bigint`, with a real FK to `accounts(id)`. Drop `C1081C`/`C1082C` — derive the display code. |
| `Base.BackupDir` | gone; backup is an operator/`pg_dump` concern (§10). |
| `Anbar_Config` | `warehouses` table with the six `*_account_id bigint REFERENCES accounts(id)` columns and `default_tax_rate numeric(5,2) NOT NULL CHECK (default_tax_rate >= 0 AND default_tax_rate <= 100)`. |
| Registry | gone. |

Cross-cutting rules for the rebuild:

1. **Secrets never live in a file the application writes.** Connection strings and keys come from
   the environment; the application has no code path that persists them.
2. **Every setting has a declared type and a seeded default**, in a migration. No lazy creation, no
   label-as-value, no silent no-op writes.
3. **Separate the three scopes explicitly**: instance config (env), tenant/organisation config
   (`app_settings`, `organization`), and user preferences (`user_preferences`). The legacy design
   mixes all three in one ini file and one `Base` row.
4. Settings that change **accounting** behaviour (`C1081`, `C1082`, `AC_*` account mappings,
   `AC_DMaliat`) must be **audited** — `updated_by`, `updated_at`, and an append-only change log —
   because they silently alter what future postings do. The legacy system records none of this.


---

[← 02-08-a-configuration-ini-and-tanzim.md](02-08-a-configuration-ini-and-tanzim.md) | [02-09-concurrency-locking-and-transactions.md →](02-09-concurrency-locking-and-transactions.md)
