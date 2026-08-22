_Part of [02-data-model](../02-data-model.md) — [index](00-index.md)_

### 2.6 `Sahamdar` → `parties`

**Purpose.** The **person and legal-entity register** — *not* a shareholder/equity table
(`docs/01-glossary.md` §6b). `S_Kind = 1` natural person, `S_Kind = 2` legal entity. **Not
year-scoped.** A party gets a chart-of-accounts node via `Sarfasl_Add(Ko, Mo, S_Card, 0, name)`
(`SahamdarEditU.pas:294-297`) — i.e. **`Sahamdar.S_Card` becomes the `Sarfasl.S_Ta1` segment**, which
is the only link between the two tables and it is positional, not a foreign key.

Column list is authoritative — it comes from the `Sahamdar_Edit` stored-procedure signature
(`Dmu.dfm:173-307`, confidence **A** for those columns) plus the `UPDATE` at
`SahamdarEditU.pas:296-312`.

| Legacy column | Legacy type | Proposed column | PostgreSQL type | Null | Default | Identity | Business meaning |
|---|---|---|---|---|---|---|---|
| `S_SSN` | `int IDENTITY` | `id` | `bigint GENERATED ALWAYS AS IDENTITY` | no | — | **yes** | Surrogate PK. Read by `Sahamdar_Show(@Id)`. |
| `S_Card` | `int` | `card_number` | `integer` | no | — | no | **The business key** — member/card number. Used by `Sahamdar_Seek(@S_card)`, by `Is_Admin_Or_Valid_Jari` (`Dmu.pas:975`) and as the `Sarfasl.S_Ta1` segment. Duplicate detection is on this **and** on the national ID (`SahamdarEditU.pas:273-279`, message `کدملي تکراري است` — *"duplicate national ID"*). |
| `S_Kind` | `tinyint`/`smallint` (`ftWord`) | `party_type` | `smallint` | no | `1` | no | `1` = natural person (`حقیقی`), `2` = legal entity (`حقوقی`). Hard-coded `1` on insert (`SahamdarEditU.pas:290`). |
| `S_Name` | `varchar(50)` | `first_name` | `text` | no | — | no | Given name |
| `S_Famil` | `varchar(50)` | `last_name` | `text` | yes | — | no | Surname (`فامیل`) |
| `S_Father` | `varchar(50)` | `father_name` | `text` | yes | — | no | Father's name (`نام پدر`) |
| `S_BDate` | `char(8)` | `birth_date` | `date` | yes | — | no | Date of birth, **8-char Jalali** per the SP parameter — the short form (§6.1) |
| `S_BPlace` | `varchar(20)` | `birth_place` | `text` | yes | — | no | `محل تولد` |
| `S_SDate` | `char(8)` | `id_issue_date` | `date` | yes | — | no | ID-card issue date, 8-char Jalali |
| `S_SPlace` | `varchar(20)` | `id_issue_place` | `text` | yes | — | no | `محل صدور` |
| `S_IDNO` | `int` | `id_card_number` | `text` | yes | — | no | ID-card number (`شماره شناسنامه`). **Declared `int`** — leading zeros are destroyed. Migrate to `text`. |
| `S_Address` | `varchar(100)` | `address` | `text` | yes | — | no | |
| `S_CodeMelli` | `char(10)` | `national_id` | `text` | yes | — | no | National ID. Uniqueness enforced only in the UI (`SahamdarEditU.pas:273-279`). |
| `S_CodePosti` | `char(10)` | `postal_code` | `text` | yes | — | no | |
| `S_CodeSabt` | `varchar` | `registration_number` | `text` | yes | — | no | Company registration number (legal entities). `SahamdarEditU.pas:308` |
| `S_Mobile` | `varchar(12)` | `mobile` | `text` | yes | — | no | |
| `S_Phone` | `varchar(12)` | `phone` | `text` | yes | — | no | |
| `S_Siba` | `varchar(13)` | `bank_account_siba` | `text` | yes | `' '` | no | SIBA (Bank Melli) account number. **Server default is a single space, not `NULL`** (`Dmu.dfm` param default `' '`). |
| `S_ShabaNo` | `varchar(26)` | `iban` | `text` | yes | — | no | IBAN / SHEBA. Validated by `TDM.IsValidShaba` (`Dmu.pas:196-214`, ISO 13616 mod-97). |
| `S_MaliatState` | `int` | `tax_status` | `smallint` | no | `0` | no | Tax status; the value is a **combo-box `ItemIndex`** written straight to the column (`SahamdarEditU.pas:309`) — so the meaning of each integer lives only in the `.dfm` item list (§12). |
| `S_Shanas` | `varchar` | `entity_national_id` | `text` | yes | — | no | Legal-entity national ID (`شناسه ملی`). `SahamdarP.pas:98` |
| `S_Lock` | `int` | `is_locked` | `boolean` | no | `false` | no | Administrative freeze; checked by `Is_Admin_Or_Valid_Jari` (`Dmu.pas:979`), **fail-open** for an unknown card (§9.6). UI toggle `SahamdarU.pas:78`. |
| `S_Aks` / `S_AKS` | `image` | `photo` | `bytea` | yes | — | no | Party photograph. Scanned images also live **outside** the database, at `\\pesteh\SahamData\<card>\…` (§1.5, `CardJariU.pas:329-331`). |

**Inferred keys.** PK `S_SSN`; **unique `S_Card`**; **unique `S_CodeMelli`** where not null (the UI
enforces both, `SahamdarEditU.pas:265-279`). No FK to `Sarfasl` exists — the relationship is
`Sarfasl.S_Ta1 = Sahamdar.S_Card` **within a particular `(S_Ko, S_Mo)` pair**, which is why
`SahamdarEditU.pas:288-300` loops over a `Coding` dataset creating one account per Kol/Moein pair.
**Model this explicitly in the rebuild** (`accounts.party_id bigint REFERENCES parties(id)`) rather
than reproducing the positional encoding — see §13.

---

### 2.7 `Moein` → `voucher_lines`

**Purpose.** The **general journal line** table — the heart of the system. One row per debit or
credit posting. Despite the name (`معین` = subsidiary account), this is *not* a subsidiary ledger:
it is the single line table for every voucher in the system.

Confidence **A** for the typed columns (persistent fields in `DKolU.dfm`, `DMoein.dfm`,
`DaftarT_U.dfm`, `MakeSanadU.dfm`, `MoeinSearchU.dfm`, `Report7U.dfm`, `SanadEditU.dfm`).

| Legacy column | Legacy type | Proposed column | PostgreSQL type | Null | Default | Identity | Business meaning |
|---|---|---|---|---|---|---|---|
| `M_SSN` | `int IDENTITY` (`TAutoIncField`) | `id` | `bigint GENERATED ALWAYS AS IDENTITY` | no | — | **yes** | Surrogate PK |
| `M_COID` | `int` | `fiscal_year_id` | `bigint` | no | — | no | **Fiscal-year stamp** (§1.4). Casing varies: `M_COID`, `M_Coid`, `M_CoID` across queries. |
| `M_Sanad` | `int` | `voucher_number` | `integer` | no | — | no | Voucher number within the year. Allocated `MAX+1` (`Dmu.pas:1247`, §5.3.1). **Logical FK → `DMoein.DM_Sanad`** on `(M_COID, M_Sanad)` — not enforced. |
| `M_Date` | `char(10)` (`TStringField Size=10`) | `line_date` | `date` | no | — | no | Jalali `'YYYY/MM/DD'`. Should equal the voucher header date; `TDM.Get_SanadDate` reads `MAX(M_Date)` per voucher (`Dmu.pas:1486`), which only makes sense because they are supposed to be identical. |
| `M_Bed` | `bigint` (`TLargeintField`) | `debit_amount` | `bigint` | no | `0` | no | Debit, whole rial, always ≥ 0 (§7.2) |
| `M_Bes` | `bigint` (`TLargeintField`) | `credit_amount` | `bigint` | no | `0` | no | Credit, whole rial, always ≥ 0. Exactly one of `M_Bed`/`M_Bes` is non-zero. |
| `M_Ted` | `decimal(18,3)` (`TBCDField p18 s3`) | `quantity` | `numeric(18,3)` | no | `0` | no | Quantity (`تعداد`) — the ledger carries a quantity alongside the amount. Also seen as `TSingleField` and `TStringField(10)` in older forms — **type drift; confirm** (§12). |
| `Article` | `varchar(250)` (`TStringField Size=250`) | `description` | `text` | yes | — | no | Line narration (`آرتیکل`). Note: **no `M_` prefix.** Also declared as `M_Article varchar(250)` in `SanadEditU.dfm` — two names for what may be one column (§12). |
| `M_Ko` | `int` | `general_ledger_code` | `integer` | no | `0` | no | Denormalised copy of `Sarfasl.S_Ko` |
| `M_Mo` | `int` | `subsidiary_code` | `integer` | no | `0` | no | Denormalised `Sarfasl.S_Mo` |
| `M_Ta1` | `int` | `analytic1_code` | `integer` | no | `0` | no | Denormalised `Sarfasl.S_Ta1` |
| `M_Ta2` | `int` | `analytic2_code` | `integer` | no | `0` | no | Denormalised `Sarfasl.S_Ta2` |
| `M_Code` | `int` | `account_id` | `bigint` | no | — | no | **FK → `Sarfasl.S_SSN`.** Back-filled after insert in several paths (`FactorPesteh_U.pas:229-230`) and left `0` by the file importer (`SanadMoeinu.pas:328`, §10.6 defect). Also appears as `TStringField` in `MakeSanagU.dfm` — type drift. |
| `M_Tx` | `int` | `status` | `voucher_status` enum | no | `'draft'` | no | `0` draft / `1` confirmed / `2` posted (§9.7). Duplicated from `DMoein.DM_Tx`. |
| `M_Kind` | `smallint` (`TSmallintField`) | `journal_kind` | `smallint` | no | `1` | no | `1` = ledger (`Moein`), `2` = journal/daybook (`Rooznameh`). Only these two values are written (`ArticleMoeinu.dfm` `@kind` default `1`, `ArticleRooznamehU.dfm` default `2`). |
| `M_ID` | `int` | `source_module` | `smallint` | no | `0` | no | **Source-module code** — which subsystem created the line. Observed values below. |
| `M_Link` | `int` | `source_id` | `bigint` | yes | — | no | Primary key of the source document **in the table implied by `M_ID`** — a polymorphic pointer, impossible to constrain. E.g. `M_ID=21` ⇒ `M_Link = DCheck.S_SSN` (`CheckDaryaftU.pas:333`). |
| `M_User` | `int` | `created_by` | `bigint` | no | — | no | **FK → `PassWord.UserCode`.** Hard-coded to `68` by the carry-forward routine (`EnteghalU.pas:254` etc., §10.5 defect 3). |
| `M_Time` | `datetime` (`TDateTimeField`) | `created_at` | `timestamptz` | no | `now()` | no | Server audit stamp, written as `GetDate()` (`MakeSanadU.pas:93`, `SanadEditU.pas:630`, `CheckEditU.pas:486`, `TankhahEdit.pas:463`, `FactorPesteh_U.pas:224,226`). **Not** written by every path. |
| `M_L`, `M_R` | `varchar(25…50)` | *(dropped — derive)* | — | — | — | — | Sort keys joined in from `Sarfasl` in some result sets; whether they are physical columns *on `Moein`* is unclear (§12). |
| `M_Name` | `varchar(200…250)` | *(dropped — derive)* | — | — | — | — | Denormalised account name in result sets. |
| `M_CR`, `M_CodeStr` | `varchar(25…50)` | *(dropped — derive)* | — | — | — | — | Denormalised formatted account code in result sets. |

**`M_ID` — source-module codes observed in the source.** This is an enumeration with no definition
anywhere; the list below is what the code writes or filters on, and is **certainly incomplete**
(§12).

| `M_ID` | Written / filtered at | Meaning |
|---|---|---|
| `1` | `AnbarFactorU.pas:593` (in the `'1,2,3,4,5,6,7,8,9'` list) | inventory invoice family |
| `2`–`9` | same list | further inventory document types |
| `15` | grep `M_ID=15` | — (unidentified) |
| `21` | `CheckDaryaftU.pas:327,333`; `Dmu` | **cheque received** (`چک دریافتی`) |
| `22` | `CheckBargashtu.pas:229,241` | **cheque bounced / returned from bank** |
| `23`, `24` | cheque family (`'21,…,29'`, `CheckBargashtu.pas:197`) | cheque collected / returned to issuer |
| `25` | `FISHDaryaftU.pas:185` | **bank deposit slip** (`فیش`) |
| `26` | `CheckEditU.pas:486` | **cheque payment document** (`CheckMaster`) |
| `27`–`29` | cheque family list | further treasury events |
| `34` | `FactorPesteh_U.pas:224,226` | **pistachio purchase receipt → invoice** |
| `35` | grep `M_ID = 35` | — (unidentified) |
| `41` | `TankhahEdit.pas:463` | **petty cash** (`تنخواه`) |
| `68` | *(never written as `M_ID`)* | — see the `EnteghalU` bug: `68` lands in `M_User`, not `M_ID` |

**Inferred keys.**

- PK `M_SSN`.
- FK `M_Code → Sarfasl(S_SSN)` — **not enforced**; `SanadMoeinu.pas:328` writes `0`.
- FK `(M_COID, M_Sanad) → DMoein(DM_Coid, DM_Sanad)` — **not enforced**; the two tables are kept in
  step by application code only (`TDM.DMoein_Make`, `Dmu.pas:828-838`).
- FK `M_COID → Base(CO_ID)` — not enforced.
- FK `M_User → PassWord(UserCode)` — not enforced.
- `(M_ID, M_Link)` is a **polymorphic** pointer — cannot be a FK; see §13 for the proposal to
  replace it with per-source nullable FKs or a link table.

**Indexes the query patterns demand** (none observable in source): `(M_COID, M_Sanad)`,
`(M_COID, M_Date)`, `(M_COID, M_Ko, M_Mo, M_Ta1, M_Ta2, M_Date)`, `(M_ID, M_Link)`, `(M_Code)`.

**Constraints proposed** (§11): `CHECK (debit_amount >= 0 AND credit_amount >= 0)`,
`CHECK (debit_amount = 0 OR credit_amount = 0)`, `CHECK (quantity >= 0)`.

---

### 2.8 `DMoein` → `vouchers`

**Purpose.** The **voucher header** (`سند`). One row per `(fiscal year, voucher number)`, carrying
denormalised totals and the workflow state. Created and maintained exclusively by
`TDM.DMoein_Make` (`Dmu.pas:828-838`) and `TDM.Dmoein_UpdateMab`.

Confidence **A** (persistent fields in `RooznamehViewU.dfm`, `SanadViewU.dfm`).

| Legacy column | Legacy type | Proposed column | PostgreSQL type | Null | Default | Identity | Business meaning |
|---|---|---|---|---|---|---|---|
| `DM_SSN` | `int IDENTITY` (`TAutoIncField`) | `id` | `bigint GENERATED ALWAYS AS IDENTITY` | no | — | **yes** | Surrogate PK |
| `DM_Coid` | `int` | `fiscal_year_id` | `bigint` | no | — | no | Fiscal-year stamp |
| `DM_Sanad` | `int` | `voucher_number` | `integer` | no | — | no | Voucher number within the year. Allocated `MAX+1` from **`DMoein`** here (`MoeinToRU.pas:264`) but from **`Moein`** elsewhere (`Dmu.pas:1247`) — §5.3.1 defect. |
| `DM_Date` | `char(10)` (`TStringField Size=10`) | `voucher_date` | `date` | no | — | no | Jalali voucher date |
| `DM_Desc` | `varchar(500)` (`TStringField Size=500`) | `description` | `text` | yes | — | no | Voucher narration |
| `DM_TBed` | `bigint` (`TLargeintField`) | `total_debit` | `bigint` | no | `0` | no | **Denormalised** `SUM(Moein.M_Bed)`. Recomputed by `Dmoein_UpdateMab`, outside any transaction (§9.4). |
| `DM_TBes` | `bigint` (`TLargeintField`) | `total_credit` | `bigint` | no | `0` | no | **Denormalised** `SUM(Moein.M_Bes)` |
| `DM_Count` | `int` | `line_count` | `integer` | no | `0` | no | **Denormalised** line count. A header with `DM_Count = 0` is deleted (`Dmu.pas:855`). |
| `DM_Tx` | `tinyint` (`TWordField`) | `status` | `voucher_status` enum | no | `'draft'` | no | `0` draft / `1` confirmed / `2` posted. Transition `0→1` requires `DM_TBed = DM_TBes` (`SanadViewU.pas:298,301`) — **the only balance check in the system.** |
| `DM_Kind` | `int` | `journal_kind` | `smallint` | no | `1` | no | `1` = ledger, `2` = journal/daybook (`RooznamehViewU.pas:139` filters `DM_kind=2`). Mirrors `Moein.M_Kind`. |
| `DM_Lock` | `tinyint` (`TWordField`) | `is_locked` | `boolean` | no | `false` | no | Administrative freeze; non-admins blocked by `Is_Admin_Or_Valid_Sanad` (`Dmu.pas:993`), **fail-closed** (§9.6). UI toggle `RooznamehViewU.pas:420`, permission key `1139`. |
| `DM_Atf` | `int` | `attachment_count` | `integer` | yes | — | no | `عطف` — cross-reference / attachment number. Purpose unconfirmed (§12). |
| `DM_CUser` | `int` | `created_by` | `bigint` | no | — | no | **FK → `PassWord.UserCode`.** `Dmu.pas:831` |
| `DM_CDate` | `datetime` (`TDateTimeField`) | `created_at` | `timestamptz` | no | `now()` | no | `GetDate()` (`Dmu.pas:831,835`) |
| `DM_MUser` | `int` | `updated_by` | `bigint` | yes | — | no | Last modifier |
| `DM_MDate` | `datetime` (`TDateTimeField`) | `updated_at` | `timestamptz` | yes | — | no | Last modification stamp |

**Note the confusing naming**: `DM_CUser`/`DM_CDate` are written on **update** at `Dmu.pas:831`
(`Update DMoein Set DM_CUser=@User, DM_CDate=GetDate(), …`) and `DM_MUser`/`DM_MDate` on **insert**
at `Dmu.pas:834-835`. The C/M prefixes are **swapped** relative to the usual create/modify
convention. Verify against live data before mapping to `created_at`/`updated_at`.

**Inferred keys.** PK `DM_SSN`; **unique `(DM_Coid, DM_Sanad)`** — required by the whole design,
almost certainly not enforced (§5.6 R8). FK `DM_Coid → Base(CO_ID)`.

**Constraint proposed** (§11): `CHECK (total_debit = total_credit)` **deferred**, or enforced only
for `status <> 'draft'` — matching `SanadViewU.pas:298`.


---

[← 02-02-a-table-inventory-overview.md](02-02-a-table-inventory-overview.md) | [02-03-a-stored-procedures-overview.md →](02-03-a-stored-procedures-overview.md)
