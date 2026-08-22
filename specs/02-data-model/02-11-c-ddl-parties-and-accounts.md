_Part of [02-data-model](../02-data-model.md) — [index](00-index.md)_

### 11.3 Parties and the chart of accounts

Both tables carry no `fiscal_year_id` — not year-scoped, exactly as in the legacy (§1.4). Both
**do** carry `tenant_id` (§A3, §11.0 convention 1): each client has their own party register and
their own chart of accounts. "Global" in the legacy sense (not per fiscal year) is not the same as
global across clients — the earlier "stays global" language in §11.0 predates the A3 ruling and no
longer applies to tenancy.

```sql
-- =====================================================================
-- 11.3  Parties and chart of accounts
-- Legacy: Sahamdar, Sarfasl
-- =====================================================================

-- ---------------------------------------------------------------------
-- parties   <- legacy Sahamdar   §2.6
-- NOT a shareholder/equity table (01-glossary.md §6b) — a person and legal-entity register.
-- ---------------------------------------------------------------------
CREATE TABLE parties (
    id                  bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,   -- legacy S_SSN
    tenant_id           bigint      NOT NULL REFERENCES tenants(id),      -- [NEW] §A3
    card_number         integer     NOT NULL,        -- legacy S_Card — THE business key; also becomes accounts.analytic1_code
    party_type          party_type  NOT NULL DEFAULT 'natural_person',     -- legacy S_Kind 1/2
    first_name          text        NOT NULL,        -- legacy S_Name    varchar(50)   نام
    last_name           text,                        -- legacy S_Famil   varchar(50)   فامیل
    father_name         text,                        -- legacy S_Father  varchar(50)   نام پدر
    birth_date          date,                        -- legacy S_BDate   char(8) Jalali SHORT form
    birth_place         text,                        -- legacy S_BPlace  varchar(20)   محل تولد
    id_issue_date       date,                        -- legacy S_SDate   char(8) Jalali SHORT form
    id_issue_place      text,                        -- legacy S_SPlace  varchar(20)   محل صدور
    id_card_number      text,                        -- legacy S_IDNO int — see COMMENT
    address             text,                        -- legacy S_Address varchar(100)
    national_id         text,                        -- legacy S_CodeMelli  char(10)   کد ملی
    postal_code         text,                        -- legacy S_CodePosti  char(10)
    registration_number text,                        -- legacy S_CodeSabt              شماره ثبت
    entity_national_id  text,                        -- legacy S_Shanas               شناسه ملی
    mobile              text,                        -- legacy S_Mobile  varchar(12)
    phone               text,                        -- legacy S_Phone   varchar(12)
    bank_account_siba   text,                        -- legacy S_Siba    varchar(13), server default ' ' not NULL
    iban                text,                        -- legacy S_ShabaNo varchar(26)
    tax_status          smallint    NOT NULL DEFAULT 0,   -- legacy S_MaliatState — a combo ItemIndex written raw (§12.9)
    is_locked           boolean     NOT NULL DEFAULT false,   -- legacy S_Lock
    photo               bytea,                       -- legacy S_Aks image
    -- MIGRATION ONLY (§6.8 rule 4, §12.1)
    legacy_birth_date_jalali     text,
    legacy_id_issue_date_jalali  text,
    created_at          timestamptz NOT NULL DEFAULT now(),
    updated_at          timestamptz,
    created_by          bigint      REFERENCES users(id),
    updated_by          bigint      REFERENCES users(id),
    CONSTRAINT parties_card_number_key UNIQUE (tenant_id, card_number),            -- changed from a global UNIQUE — §A3
    CONSTRAINT parties_first_name_nonblank CHECK (length(btrim(first_name)) > 0),  -- [AS-IS] SahamdarEditU.pas
    CONSTRAINT parties_national_id_digits
        CHECK (national_id IS NULL OR national_id ~ '^[0-9]{10}$'),                -- [NEW]
    CONSTRAINT parties_iban_format
        CHECK (iban IS NULL OR iban ~ '^IR[0-9]{24}$')                             -- [AS-IS] TDM.IsValidShaba (Dmu.pas:196-214)
);

CREATE INDEX parties_tenant_idx ON parties (tenant_id);

ALTER TABLE parties ENABLE ROW LEVEL SECURITY;
ALTER TABLE parties FORCE ROW LEVEL SECURITY;
CREATE POLICY parties_tenant_isolation ON parties
    USING (tenant_id = current_setting('app.tenant_id')::bigint)
    WITH CHECK (tenant_id = current_setting('app.tenant_id')::bigint);

-- Unique national ID, but only where present — the legacy allows blanks and enforces
-- uniqueness only in the UI (SahamdarEditU.pas:273-279, message 'کدملي تکراري است').
-- Scoped per tenant: two different clients may legitimately share a counterparty's national ID.
CREATE UNIQUE INDEX parties_national_id_key                                        -- [NEW] §12.11
    ON parties (tenant_id, national_id) WHERE national_id IS NOT NULL AND btrim(national_id) <> '';

CREATE INDEX parties_last_name_idx  ON parties (tenant_id, last_name, first_name);
CREATE INDEX parties_mobile_idx     ON parties (tenant_id, mobile) WHERE mobile IS NOT NULL;

COMMENT ON COLUMN parties.id_card_number IS
  'Legacy S_IDNO is declared bigint in the table DDL (Full_Script_14050527.sql, 2026-08-19 — '
  'corrected from an earlier ftInteger-parameter-based assumption, Dmu.dfm:173-307), so Iranian '
  'ID numbers with leading zeros are already CORRUPTED in the legacy data regardless. Stored as '
  'text here; the migration cannot recover the lost zeros.';
COMMENT ON COLUMN parties.tax_status IS
  'Legacy S_MaliatState — the raw ItemIndex of a combo box (SahamdarEditU.pas:309). The meaning of '
  'each integer lives ONLY in the .dfm item list. NOT an enum until §12.9 recovers the list.';
COMMENT ON COLUMN parties.bank_account_siba IS
  'Legacy S_Siba. The SERVER DEFAULT is a single space, not NULL (Dmu.dfm param default '' ''). '
  'Migration should normalise '' '' to NULL.';
COMMENT ON COLUMN parties.photo IS
  'Legacy S_Aks. Note scanned images ALSO live outside the database at \\pesteh\SahamData\<card>\ '
  '(§1.5, CardJariU.pas:329-331) — those are not migrated by this table.';

-- ⚠ DISCREPANCY found 2026-08-19 against a schema-only dump (Full_Script_14050527.sql). The
-- legacy Sahamdar table's FULL column list there is only:
--   S_SSN, S_Card int NOT NULL PRIMARY KEY, S_Name varchar(50), S_Famil varchar(50),
--   S_Father varchar(50), S_IDNO bigint, S_Mobile varchar(12), S_BDate varchar(10),
--   S_BPlace varchar(50), S_SDate varchar(10), S_SPlace varchar(20), S_Address varchar(100),
--   S_CodeMelli varchar(12) DEFAULT ((0)), S_CodePosti varchar(12), S_Melli varchar(20),
--   S_keshavarzi varchar(20), S_Kind int NOT NULL DEFAULT (1), S_Lock tinyint NOT NULL DEFAULT (0),
--   S_CodeSabt varchar(16), S_MaliatState tinyint.
-- Against the table above: S_BDate/S_SDate are varchar(10), NOT char(8) short-form as claimed here
-- and in 02-12-a.md §12.1's "several SP parameters are size 8" note — CORRECTED, these two are
-- long-form like every other date column. S_IDNO is bigint, not int (the COMMENT below claiming
-- "declared INTEGER" needs correction too — bigint does not truncate leading-zero IDs the way a
-- 32-bit int would, though text storage still can't recover zeros already lost upstream).
-- S_CodeMelli/S_CodePosti are varchar(12), not char(10). AND: S_Shanas, S_Phone, S_Siba,
-- S_ShabaNo, S_Aks — five columns this `parties` design assumes — are NOT in this dump's Sahamdar
-- DDL at all. Conversely, S_Melli varchar(20) and S_keshavarzi varchar(20) exist in the dump and
-- are NOT mapped to any column in this `parties` design. Do not silently reconcile this — the
-- dump could be an older/trimmed snapshot, or the five assumed columns could be a documentation
-- error; either way it needs a fresh dump or a source-code recheck before this table is finalised.
-- Tracked as 11-open-decisions.md A16.

-- Legacy → Rust:
--   Sahamdar_Edit (Dmu.dfm:173-307)  → parties::upsert()      — SP body required (§12.3 item 8)
--   Sahamdar_Seek (Dmu.dfm:150-172)  → parties::get_by_card()
--   Sahamdar_Show (Dmu.dfm:568)      → parties::get()          — note @Id vs @Card (§12.15 item 3)
--   TDM.IsValidShaba (Dmu.pas:196)   → validation::iban()


-- ---------------------------------------------------------------------
-- accounts   <- legacy Sarfasl   §2.5
-- The chart of accounts AND the counterparty master. No fiscal_year_id (not year-scoped), but
-- tenant_id IS required (§A3) — each client has their own chart of accounts.
-- ---------------------------------------------------------------------
CREATE TABLE accounts (
    id                  bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,   -- legacy S_SSN
    tenant_id           bigint  NOT NULL REFERENCES tenants(id),           -- [NEW] §A3
    general_ledger_code integer NOT NULL DEFAULT 0,   -- legacy S_Ko   کل        (seed 111, SNewu.pas:553)
    subsidiary_code     integer NOT NULL DEFAULT 0,   -- legacy S_Mo   معین      0 ⇒ node IS a Kol
    analytic1_code      integer NOT NULL DEFAULT 0,   -- legacy S_Ta1  تفصیلی ۱  0 ⇒ node is a Moein
    analytic2_code      integer NOT NULL DEFAULT 0,   -- legacy S_Ta2  تفصیلی ۲  0 ⇒ node is a Tafsil1
    name                text    NOT NULL,             -- legacy S_Name varchar(100) — CONFIRMED (§12.5, was "100…200 unconfirmed")
    child_count         integer NOT NULL DEFAULT 0,   -- legacy S_Child — denormalised; 0 ⇒ leaf ⇒ postable
    is_locked           boolean NOT NULL DEFAULT false,  -- legacy S_Lock — checked HIERARCHICALLY (Dmu.pas:920-969)
    is_active           boolean NOT NULL DEFAULT true,   -- legacy S_Active — RESOLVED (§12.10 item 6): Active_Set's body
                                                          -- sets S_Active=1 iff S_Child=0 AND S_Mo>0 (leaf, below Kol level)
    -- Counterparty attributes (a customer/supplier IS a leaf node — 01-glossary.md §6b)
    address             text,                         -- legacy S_Address   Sarfasl_TakmilU.pas:67
    phone               text,                         -- legacy S_Tel       :69
    fax                 text,                         -- legacy S_Fax       :70
    registration_number text,                         -- legacy S_Sabt      :68
    economic_code       text,                         -- legacy S_Egh       :71
    postal_code         text,                         -- legacy S_Post      :72
    national_id         text,                         -- legacy S_Melli     :73
    -- [NEW] §13.6 / §13.7 — commented out until approved.
    -- parent_id        bigint REFERENCES accounts(id),
    -- level            smallint NOT NULL,
    -- party_id         bigint REFERENCES parties(id),
    created_at          timestamptz NOT NULL DEFAULT now(),
    updated_at          timestamptz,
    created_by          bigint      REFERENCES users(id),
    updated_by          bigint      REFERENCES users(id),
    CONSTRAINT accounts_natural_key   -- [AS-IS], not new — see note below
        UNIQUE (tenant_id, general_ledger_code, subsidiary_code, analytic1_code, analytic2_code),  -- changed from a global UNIQUE — §A3
    CONSTRAINT accounts_name_nonblank  CHECK (length(btrim(name)) > 0),                 -- [AS-IS] SNewu.pas
    CONSTRAINT accounts_segments_nonneg
        CHECK (general_ledger_code >= 0 AND subsidiary_code >= 0
               AND analytic1_code >= 0 AND analytic2_code >= 0),                        -- [NEW]
    CONSTRAINT accounts_child_count_nonneg CHECK (child_count >= 0),                    -- [NEW]
    -- The hierarchy is positional: a deeper segment may not be set unless every shallower one is.
    CONSTRAINT accounts_segment_hierarchy
        CHECK ( (subsidiary_code = 0 AND analytic1_code = 0 AND analytic2_code = 0)
             OR (subsidiary_code > 0 AND analytic1_code = 0 AND analytic2_code = 0)
             OR (subsidiary_code > 0 AND analytic1_code > 0 AND analytic2_code = 0)
             OR (subsidiary_code > 0 AND analytic1_code > 0 AND analytic2_code > 0) )   -- [NEW] see COMMENT
);

-- RESOLVED 2026-08-19 (02-12-a.md §12.6): the legacy Sarfasl table ALREADY enforces this natural
-- key with a real composite PRIMARY KEY (CONSTRAINT PK_Sarfasl_1 PRIMARY KEY (S_Ko,S_Mo,S_Ta1,S_Ta2))
-- — one of only two enforced constraints in the entire legacy schema. accounts_natural_key above
-- is [AS-IS], reproducing an existing guarantee, not [NEW] as its neighbouring comment implies.
-- Also confirmed: three legacy NONCLUSTERED indexes exist on Sarfasl(S_IS_ADaryafti),
-- Sarfasl(S_IS_Check) and Sarfasl(S_IS_Fish) — single-column, one per flag. Since those four
-- S_IS_* columns are deliberately NOT modelled in this `accounts` table (per §12.10 item 7, they
-- are unmaintained by any procedure in the legacy dump), those three legacy indexes have no
-- equivalent here — noted so their absence isn't mistaken for an oversight.
CREATE INDEX accounts_gl_idx     ON accounts (tenant_id, general_ledger_code, subsidiary_code);
CREATE INDEX accounts_name_idx   ON accounts (tenant_id, name);
CREATE INDEX accounts_leaf_idx   ON accounts (tenant_id, id) WHERE child_count = 0;
-- Trigram index for the Sarfasl_Seek_Name partial-name lookup (§3.1):
-- CREATE EXTENSION IF NOT EXISTS pg_trgm;
-- CREATE INDEX accounts_name_trgm_idx ON accounts USING gin (name gin_trgm_ops);

ALTER TABLE accounts ENABLE ROW LEVEL SECURITY;
ALTER TABLE accounts FORCE ROW LEVEL SECURITY;
CREATE POLICY accounts_tenant_isolation ON accounts
    USING (tenant_id = current_setting('app.tenant_id')::bigint)
    WITH CHECK (tenant_id = current_setting('app.tenant_id')::bigint);

-- FK from §11.2, now that accounts exists:
ALTER TABLE system_accounts
    ADD CONSTRAINT system_accounts_account_id_fkey
    FOREIGN KEY (account_id) REFERENCES accounts(id);                                  -- [NEW] §2.3, §2.4

COMMENT ON TABLE accounts IS
  'Legacy Sarfasl. NOT year-scoped (§1.4), but IS tenant-scoped (§A3) — each client has their own '
  'chart of accounts, not a shared global one. Fixed 4-level hierarchy Kol → Moein → Tafsil1 → '
  'Tafsil2 encoded positionally in four integers; a node is a leaf when child_count = 0.';
COMMENT ON CONSTRAINT accounts_segment_hierarchy ON accounts IS
  '[NEW] Formalises the implicit rule that the segments fill left to right. VERIFY against live '
  'data before adopting — a single hand-created account violating it will block the migration.';
COMMENT ON COLUMN accounts.child_count IS
  'Legacy S_Child, maintained by TDM.Update_Sarfasl_Child (Dmu.pas:300-318) and presumably by the '
  'Sarfasl_ADD / Active_Set procedures whose bodies are missing (§12.3). §13.6 proposes deriving it.';

-- DROPPED legacy columns, and why:
--   FullName  — denormalised full path; ALL maintenance code is commented out (Dmu.pas:283-296),
--               so it is stale in production. Derive.
--   M_L, M_R  — sort keys from dbo.Make_L / dbo.Make_R; maintenance disabled at Dmu.pas:274,
--               yet still read for ORDER BY. Derive (§13.6, §12.4).
--   S_IS_Check, S_IS_Fish, S_IS_APArdakhti, S_IS_ADaryafti
--             — role flags; all four assignments are commented out (Sarfasl_TakmilU.pas:76-83)
--               and the role mapping moved to Base_Config → system_accounts (§2.5).
--               VERIFY the columns are truly unused before dropping (§12.10 item 7).

-- Legacy → Rust:
--   Sarfasl_ADD (SNewu.dfm:908)        → accounts::create()  — SP body required (§12.3 item 5)
--   Sarfasl_Deep (ListSarfaslu.dfm:249)→ accounts::delete_checked() — §12.3 item 6
--   Active_Set (SNewu.pas:303)         → DELETED; is_active/child_count derived (§13.6)
--   Sarfasl_view / Sarfasl_Seek_SSN / Sarfasl_Seek_Name → accounts::list/get/search
--   Select_Kol / Select_moein / Select_Taf1 / Select_Taf2 → ONE endpoint,
--                                        GET /api/v1/accounts?parent=<code>
--   TDM.is_Sarfasl_Last_Deep (Dmu.pas:920) → accounts::is_leaf(), FAIL-CLOSED (§9.6 — the legacy
--                                        version fails OPEN for an unknown code)
--   dbo.Make_L / dbo.Make_R            → DELETED; ORDER BY the four segments or a recursive CTE
```

---


---

[← 02-11-b-ddl-platform.md](02-11-b-ddl-platform.md) | [02-11-d-ddl-accounting-core.md →](02-11-d-ddl-accounting-core.md)
