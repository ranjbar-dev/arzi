_Part of [02-data-model](../02-data-model.md) — [index](00-index.md)_

## 2. Table inventory

### 2.0 How this inventory was derived, and its confidence level

**No table DDL exists in this repository** (§3.0, §3.5). Every column below is reconstructed from
three sources, in descending order of reliability:

1. **Persistent field definitions in `.dfm` files.** Delphi captures these by interrogating the live
   server at design time, so `TStringField.Size`, `TBCDField.Precision`/`Size` (= scale),
   `TAutoIncField` (= `IDENTITY`), `TLargeintField` (= `bigint`), `TWordField` (= `tinyint`/
   `smallint`) are **trustworthy for the columns that appear**. Only a minority of columns have
   persistent fields.
2. **Stored-procedure parameter lists in `.dfm`** — same provenance, same reliability
   (e.g. `Sahamdar_Edit` at `Dmu.dfm:173-307` is effectively the `Sahamdar` column list).
3. **`INSERT` column lists, `UPDATE … SET` clauses and `FieldByName` calls in `.pas`.** These prove
   a column *exists* and hint at its type from the assignment used (`AsInteger`, `AsString`,
   `AsLargeInt`), but give **no** length, nullability, default or identity information.

Confidence is marked per table:

- **A** — full field metadata available from a `.dfm`; types and sizes are as stated.
- **B** — column names complete or near-complete, types inferred from usage; sizes unknown.
- **C** — only the columns this application happens to touch are known; the real table is wider.

**Nullability, defaults and identity are inferred throughout and marked as such.** Every `NOT NULL`
and `DEFAULT` in §11 is a *proposal*, not a transcription. Confirm against a live dump before
migrating.

### 2.1 Naming conventions in the legacy schema

| Legacy pattern | Meaning |
|---|---|
| `<PFX>_SSN` | surrogate primary key, `int IDENTITY(1,1)` (§5.1) |
| `<PFX>_COID` / `<PFX>_Coid` / `<PFX>_CoID` | **fiscal-year** stamp (§1.4). Casing is inconsistent across tables and even within one table's queries. |
| `<PFX>_Mab` | amount, `bigint` rial (§7) |
| `<PFX>_Date` | Jalali date, `char(10)` `'YYYY/MM/DD'` (§6) |
| `<PFX>_Desc` | free-text description/narration |
| `<PFX>_Sanad` | the voucher number the record was posted under |
| `<PFX>_UserID` / `<PFX>_User` | creating user (`PassWord.UserCode`) |
| `<PFX>_State` / `<PFX>_StateName` | numeric status + **denormalised Persian label of that status** |
| `<PFX>_LinkPrg` + `<PFX>_LinkSSN` | polymorphic pointer: source module id + source row id |
| `<PFX>_BedSSN`/`BedCR`/`BedName`, `<PFX>_BesSSN`/`BesCR`/`BesName` | debit / credit account id + **denormalised** code string + **denormalised** name |

The `*CR` and `*Name` columns are **denormalisation of `Sarfasl`** and are the single largest source
of stale data in the schema: nothing updates them when an account is renamed.

### 2.2 Master list

| # | Legacy table | Proposed PostgreSQL name | Domain | Year-scoped | Confidence |
|---|---|---|---|---|---|
| 1 | `Base` | `fiscal_years` (+ `organization`) | platform | *is* the year table | B |
| 2 | `Base_Config` | `system_accounts` | accounting | no | B |
| 3 | `Sarfasl` | `accounts` | accounting | **no** (global) | B |
| 4 | `Moein` | `voucher_lines` | accounting | yes (`M_COID`) | A |
| 5 | `DMoein` | `vouchers` | accounting | yes (`DM_Coid`) | A |
| 6 | `Sahamdar` | `parties` | parties | **no** (global) | B |
| 7 | `DCheck` | `cheques` | treasury | yes (`S_COID`) | A |
| 8 | `DCheck2` | `cheque_events` | treasury | yes (`S_Coid`) | B |
| 9 | `DFish` | `deposit_slips` | treasury | yes (`S_COID`) | A |
| 10 | `TCheck` | `cheque_types` (see §2.13) | treasury | unknown | C |
| 11 | `CheckMaster` | `cheque_payment_documents` | treasury | yes (`CM_Coid`) | A |
| 12 | `CheckDetail` | `cheque_payment_lines` | treasury | yes (`CD_Coid`) | A |
| 13 | `TankhahMaster` | `petty_cash_documents` | treasury | yes (`TM_Coid`) | A |
| 14 | `TankhahDetail` | `petty_cash_lines` | treasury | yes (`TD_Coid`) | B |
| 15 | `Anbar_Factor` | `inventory_invoices` | inventory | yes (`AF_COID`) | A |
| 16 | `Anbar_FactorD` | `inventory_invoice_lines` | inventory | yes (`AFD_Coid`) | B |
| 17 | `Anbar_Jens` | `items` | inventory | **no** | B |
| 18 | `Anbar_Config` | `warehouses` | inventory | no | B |
| 19 | `Anbar_Vahed` | `units_of_measure` | inventory | no | C |
| 20 | `Kinds` | `product_grades` | inventory (pistachio) | no | C |
| 21 | `Tanzim` | `app_settings` | platform | no | B |
| 22 | `PassWord` | `users` | platform | no | B |
| 23 | `Pass_Config` | `user_permissions` | platform | no | B |
| 24 | `TolidMaster` | `production_documents` | production | unknown | C |
| 25 | `Moadian` | `tax_submissions` | compliance | unknown | C |
| 26 | `temp_*` | — (transient) | — | — | — |
| E1 | `Anbar.dbo.Cala` | external | inventory (external) | — | C |
| E2 | `Anbar.dbo.Anbar` | external | inventory (external) | — | C |
| E3 | `Anbar.dbo.FactorMaster` | external | inventory (external) | yes (`FM_COID`) | A |
| E4 | `Anbar.dbo.FactorDetail` | external | inventory (external) | no | A |
| E5 | `Saham.dbo.NSaham` | external | shares (external) | — | C |
| E6 | `Rppc_Solution.dbo.NewRamz` | external | pistachio receipts (external) | — | A |

**Not tables**, despite looking like them in the data module: `Anbar_Tasfieh` (`Dmu.dfm:1017-1110`)
is a parameterised `TADOQuery` over `Anbar_Factor`/`DCheck`/`DFish`; `Base_Q`, `QCheck`, `QDCheck`,
`Jari_Rem`, `SahamdarConfig`, `KharidPeste_List` are likewise queries.

---

### 2.3 `Base` → `fiscal_years` + `organization`

**Purpose.** One row per **fiscal year**. The row simultaneously carries the operating entity's
letterhead, the account-code display widths and the two system-account pointers (§1.4, §8.3).
Logical key `CO_ID`. Read via `TADOTable Base` (`Dmu.dfm` — no persistent fields, confidence **B**)
and `Base_Q` (`Dmu.dfm:362-374`).

| Legacy column | Legacy type (inferred) | Proposed column | PostgreSQL type | Null | Default | Identity | Business meaning |
|---|---|---|---|---|---|---|---|
| `CO_ID` | `int` | `fiscal_years.year` | `integer` | no | — | no | **Fiscal year**, e.g. `1403`. The logical PK and the value stamped on every transactional row. |
| `FromDate` | `char(10)` | `fiscal_years.start_date` | `date` | no | — | no | Period start, Jalali `'YYYY/MM/DD'` in the legacy. `TanzimU.pas:250-257` |
| `ToDate` | `char(10)` | `fiscal_years.end_date` | `date` | no | — | no | Period end. `TanzimU.pas:259-269` |
| `IsActive` | `int` | `fiscal_years.is_active` | `boolean` | no | `true` | no | `1` = open. Anything else blocks all posting (`Dmu.pas:1008-1014`). **No screen writes this column** (§12). |
| `BackupDir` | `varchar(100)` | *(dropped)* | — | — | — | — | Backup destination directory (§10.1, §10.2). Not needed in the rebuild. |
| `Co_Name` | `varchar(100)` | `organization.name` | `text` | no | — | no | Organisation name on the letterhead. `TanzimU.pas:160-169` |
| `Co_Sub` | `varchar(100)` | `organization.subtitle` | `text` | yes | — | no | System/subtitle name (`نام سيستم`); concatenated into the year-picker label (`Dmu.dfm:369-370`). |
| `Co_Address` | `varchar(100)` | `organization.address` | `text` | yes | — | no | `TanzimU.pas:171-180`; DDL at `Dmu.pas:257` |
| `Co_Tel` | `varchar(20)` | `organization.phone` | `text` | yes | — | no | `Dmu.pas:258` |
| `Co_Fax` | `varchar(20)` | `organization.fax` | `text` | yes | — | no | `Dmu.pas:259` |
| `Co_Web` | `varchar(30)` | `organization.website` | `text` | yes | — | no | `TanzimU.pas:215-224` |
| `Co_EMail` | `varchar(30)` | `organization.email` | `text` | yes | — | no | `TanzimU.pas:236-245` |
| `Co_Sabt` | `varchar` | `organization.registration_number` | `text` | yes | — | no | Company registration number (`شماره ثبت`) |
| `Co_Melli` | `varchar` | `organization.national_id` | `text` | yes | — | no | Legal-entity national ID (`شناسه ملی`) |
| `Co_Egh` | `varchar(20)` | `organization.economic_code` | `text` | yes | — | no | Economic code (`کد اقتصادی`), `Dmu.pas:255` |
| `Co_Post` | `varchar(20)` | `organization.postal_code` | `text` | yes | — | no | `Dmu.pas:256` |
| `ARM` | `image` | `organization.logo` | `bytea` | yes | — | no | Logo (`آرم`). Explicitly **excluded** from the ABS backup (`Backup_U.pas:116-117`). |
| `No_Ko` | `int` | `account_code_format.width` (level 1) | `smallint` | no | `3` | no | Display width in digits of the Kol segment. `Dmu.pas:1200-1202` |
| `No_Mo` | `int` | level 2 | `smallint` | no | `2` | no | `Dmu.pas:1206-1208` |
| `No_Ta1` | `int` | level 3 | `smallint` | no | `3` | no | `Dmu.pas:1213-1215` |
| `No_Ta2` | `int` | level 4 | `smallint` | no | `3` | no | `Dmu.pas:1220-1222` |
| `Real_Len` | `int` | *(dropped)* | — | — | — | — | Only reference is commented out (`TanzimU.pas:135`). Dead. |
| `C1081` | `int` | `app_settings['accounting.cash_account_id']` | `bigint` | yes | — | no | **FK → `Sarfasl.S_SSN`**: the cash account (`صندوق`). `Dmu.pas:1065-1098` |
| `C1081C` | `varchar` | *(dropped — derive)* | — | — | — | — | Denormalised display code of `C1081`. `Dmu.pas:1078-1086` |
| `C1082` | `int` | `app_settings['accounting.in_transit_account_id']` | `bigint` | yes | — | no | **FK → `Sarfasl.S_SSN`**: cheques-in-transit / "current" account (`جریان`). `Dmu.pas:1100-1134` |
| `C1082C` | `varchar` | *(dropped — derive)* | — | — | — | — | Denormalised display code of `C1082`. |

**Inferred keys.** PK `CO_ID`. FK `C1081 → Sarfasl(S_SSN)`, `C1082 → Sarfasl(S_SSN)` — **not
enforced** in the legacy. Every `*_COID` column on every other table is a logical FK to
`Base(CO_ID)` — also not enforced.

**Split proposed** (§8.6): `fiscal_years` (year, dates, active flag), single-row `organization`
(letterhead), `account_code_format` (four widths), `app_settings` (the two account pointers).
Note this **changes behaviour**: today the letterhead and the code widths can differ per year. See
§13.

---

### 2.4 `Base_Config` → `system_accounts`

**Purpose.** A slot table mapping a **numeric role id** to a chart-of-accounts node. It is the
generalised form of `Base.C1081`/`C1082`: instead of two hard-coded columns, `Base_Config` holds one
row per "system account" role. Sole consumer `Sarfasl_SelectU.pas` (the account picker restricted to
a role) and `CheckDaryaftU.dfm:320`.

```pascal
Q1.SQL.Add(' Select B.* , S.* ');
Q1.SQL.Add(' From base_config as B ');
Q1.SQL.Add(' Left join Sarfasl as S on ( S.S_SSN=B.BC_SSN) ');
Q1.SQL.Add(' where BC_ID=15');                                   // Sarfasl_SelectU.pas:224-227
```

| Legacy column | Legacy type (inferred) | Proposed column | PostgreSQL type | Null | Default | Identity | Business meaning |
|---|---|---|---|---|---|---|---|
| `BC_ID` | `int` | `role_id` | `integer` | no | — | no | The role slot. Observed literals: `11`, `13`, `14`, `15`, plus a dynamic `BC_ID='+inttostr(ID)` (`Sarfasl_SelectU.pas`). |
| `BC_SSN` | `int` | `account_id` | `bigint` | no | — | no | **FK → `Sarfasl.S_SSN`** — the account bound to the role |
| `BC_Name` | `varchar` | `label_fa` | `text` | yes | — | no | Persian label of the role |
| `BC_Default` | `int` | `is_default` | `boolean` | no | `false` | no | Marks the default row when a role has several candidates |
| `BC_Enabled` | `int` | `is_enabled` | `boolean` | no | `true` | no | Soft-disable |

Roles identified from the calling methods (`Sarfasl_SelectU.pas`):

| `BC_ID` | Method | Persian | English |
|---|---|---|---|
| `14` | `init_AsnadDaryaftani` | اسناد دریافتنی | **notes receivable** |
| `15` | `init_AsnadDarJarjanvosool` | اسناد در جریان وصول | **notes in course of collection** |
| `13` | (`init_AsnadParDakhti` and neighbours) | اسناد پرداختنی | **notes payable** |
| `11` | (see `Sarfasl_SelectU.pas`) | — | further role — confirm against live data |

**Inferred keys.** PK probably `(BC_ID, BC_SSN)` or a surrogate; FK `BC_SSN → Sarfasl(S_SSN)`.
**The full set of `BC_ID` values cannot be determined from the source** — it must be read from the
live table (§12).

**Proposal.** Merge `Base_Config` and the `Base.C1081/C1082` pointers into **one** `system_accounts`
table keyed by a **readable enum** (`notes_receivable`, `notes_in_collection`, `notes_payable`,
`cash`, `cheques_in_transit`, …), with a real FK to `accounts(id)`.

---

### 2.5 `Sarfasl` → `accounts`

**Purpose.** The **chart of accounts**: a fixed 4-level hierarchy Kol → Moein → Tafsil1 → Tafsil2,
identified by four integers. A node is a leaf when `S_Child = 0`. **Not year-scoped** — there is no
`*_COID` column and every query omits the year (§1.4). It is also the **counterparty master**: a
customer/supplier *is* a leaf node, and its address, phone and tax identifiers live on the account
row (`docs/01-glossary.md` §6b, `Sarfasl_TakmilU.pas:65-84`).

Confidence **B**: no `.dfm` field list for the `Sarfasl` table itself (`Dmu.dfm:376-381` declares it
with none); columns come from `FieldByName` calls and SQL text.

| Legacy column | Legacy type (inferred) | Proposed column | PostgreSQL type | Null | Default | Identity | Business meaning |
|---|---|---|---|---|---|---|---|
| `S_SSN` | `int IDENTITY` | `id` | `bigint GENERATED ALWAYS AS IDENTITY` | no | — | **yes** | Surrogate PK. Referenced by `Moein.M_Code`, `DCheck.S_BedSSN/S_BesSSN/S_Zssn`, `Base.C1081/C1082`, `Base_Config.BC_SSN`, `Anbar_Config.AC_*`, `Anbar_Factor.AF_Customer`, `CheckMaster.CM_Code`, `TankhahMaster.TM_Code`. |
| `S_Ko` | `int` | `general_ledger_code` | `integer` | no | `0` | no | Level-1 segment (`کل`). Allocated `MAX+1` seeded at `111` (`SNewu.pas:553`). |
| `S_Mo` | `int` | `subsidiary_code` | `integer` | no | `0` | no | Level-2 (`معین`). `0` ⇒ the node *is* a Kol. Seed `111` (`SNewu.pas:563`). |
| `S_Ta1` | `int` | `analytic1_code` | `integer` | no | `0` | no | Level-3 (`تفصیلی ۱`). `0` ⇒ node is a Moein. Seed `1`. For party accounts this is the party's `Sahamdar.S_Card` (`SahamdarEditU.pas:294-296`). |
| `S_Ta2` | `int` | `analytic2_code` | `integer` | no | `0` | no | Level-4. `0` ⇒ node is a Tafsil1. Seed `1`. |
| `S_Name` | `varchar(100…200)` | `name` | `text` | no | — | no | Account name. `TStringField(100)`/`(200)` in different `.dfm`s — confirm the real width. |
| `S_Child` | `int` | `child_count` | `integer` | no | `0` | no | **Denormalised count of direct children.** `0` ⇒ leaf ⇒ postable. Maintained by `TDM.Update_Sarfasl_Child` (`Dmu.pas:300-318`) and presumably by `Sarfasl_ADD`/`Active_Set`. |
| `S_Lock` | `int` | `is_locked` | `boolean` | no | `false` | no | Administrative freeze. Checked **hierarchically** by `TDM.Is_Admin_Or_Valid_Daftar` (`Dmu.pas:920-969`); admins bypass. §9.6 |
| `FullName` | `varchar(200)` | *(dropped — derive)* | — | — | — | — | Denormalised full path `Kol/Moein/Tafsil1/Tafsil2` names. DDL at `Dmu.pas:261`; **all maintenance code is commented out** (`Dmu.pas:283-296`) so it is **stale in production**. |
| `M_L` | `varchar(25…50)` | *(dropped — derive)* | — | — | — | — | Sort key produced by `dbo.Make_L(coid, ko, mo, ta1, ta2)` (§3.2). Maintenance disabled at `Dmu.pas:274`. Still read for ordering (`SanadMoeinu.dfm`, `TajmiU.dfm`, `RoyatJU.dfm`). |
| `M_R` | `varchar(25…50)` | *(dropped — derive)* | — | — | — | — | Reverse sort key, `dbo.Make_R`. Persisted as a UI choice (`[Sarfasl_Select] MRL=M_R`, §8.1.2). |
| `S_Address` | `varchar` | `address` | `text` | yes | — | no | Counterparty address. `Sarfasl_TakmilU.pas:67` |
| `S_Tel` | `varchar` | `phone` | `text` | yes | — | no | `Sarfasl_TakmilU.pas:69` |
| `S_Fax` | `varchar` | `fax` | `text` | yes | — | no | `Sarfasl_TakmilU.pas:70` |
| `S_Sabt` | `varchar` | `registration_number` | `text` | yes | — | no | Company registration number. `Sarfasl_TakmilU.pas:68` |
| `S_Egh` | `varchar` | `economic_code` | `text` | yes | — | no | Economic code. `Sarfasl_TakmilU.pas:71` |
| `S_Post` | `varchar` | `postal_code` | `text` | yes | — | no | `Sarfasl_TakmilU.pas:72` |
| `S_Melli` | `varchar` | `national_id` | `text` | yes | — | no | National ID / `شناسه ملی`. `Sarfasl_TakmilU.pas:73` |
| `S_Active` | `int` | `is_active` | `boolean` | no | `true` | no | Recomputed by the `Active_Set` procedure (§3.1). Semantics unconfirmed (§12). |
| `S_IS_Check`, `S_IS_Fish`, `S_IS_APArdakhti`, `S_IS_ADaryafti` | `int` | *(dropped)* | — | — | — | — | Role flags — "this account may be used for cheques / deposit slips / notes payable / notes receivable". **All four assignments are commented out** (`Sarfasl_TakmilU.pas:76-83`); the role mapping moved to `Base_Config` (§2.4). Verify the columns still exist before dropping. |

**Inferred keys.**

- PK `S_SSN`.
- **Natural unique key `(S_Ko, S_Mo, S_Ta1, S_Ta2)`** — every lookup uses it
  (`Dmu.pas:1152-1156`, `EnteghalU.dfm:344-345`, `Dmu.pas:929-968`) and `is_Sarfasl_Last_Deep`
  asserts `RecordCount = 1` for a full code (`Dmu.pas:920-936`). **Almost certainly not enforced by
  a constraint** — add it (§11), after de-duplicating live data.
- Self-referencing parent link is **implicit**, encoded in the four segments rather than a
  `parent_id`. See §13 for the proposal to add an explicit `parent_id`.

---


---

[← 02-01-connection-and-runtime-topology.md](02-01-connection-and-runtime-topology.md) | [02-02-b-table-inventory-parties-vouchers.md →](02-02-b-table-inventory-parties-vouchers.md)
