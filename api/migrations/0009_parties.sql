-- Step 3.1 (docs/phase-3-parties.md §3.1 / specs/02-data-model/02-11-c-ddl-parties-and-accounts.md
-- §11.3): the party register (legacy Sahamdar) plus SahamdarConfig's control-account map, and the
-- FK from accounts.party_id (added by 2.1, left pending for this step) to parties.
--
-- Columns dropped vs. the DDL doc's earlier draft, per this step's Build bullet
-- (specs/07-parties-and-shareholders/07-04-a.md §4.1): S_Phone, S_Siba, S_Shanas — confirmed
-- unused by both live editors (SahamdarEditU.pas / CompanyEditU.pas), only ever touched by the
-- dead Sahamdar_Edit/SahamdarP forms. `iban`/`photo` likewise dropped — neither appears in
-- 07-04-a.md's live-editor column table at all (they were a speculative addition in the DDL draft,
-- predating the A16 discrepancy note); nothing in this step's Build bullet or manual test needs them.
--
-- SahamdarConfig's `SC_Tik` scratch column is NOT ported (B18 — 11-open-decisions.md) — "does this
-- party already have an account under this control account" is computed per-request in
-- api/src/parties.rs, never persisted. `SC_Kind` also dropped — 07-07.md §7.2 notes it's read only
-- by the dead `Dm.SahamdarConfig` query (§7.4c), never written by anything.

CREATE TYPE party_type AS ENUM ('natural_person', 'legal_entity');  -- legacy S_Kind 1/2

-- legacy S_MaliatState — an unlabelled combo ItemIndex in the Delphi source; the five values are
-- named here per 07-04-a.md §4.2's decoded item list (docs/phase-3-parties.md's Build bullet).
CREATE TYPE party_tax_status AS ENUM (
    'not_specified',
    'taxpayer_required_to_register',
    'natural_person_article_81',
    'not_required_to_register',
    'final_consumer'
);

-- ---------------------------------------------------------------------
-- parties   <- legacy Sahamdar
-- Not year-scoped (§1.4 — global per-tenant master data), but IS tenant-scoped (§A3).
-- ---------------------------------------------------------------------
CREATE TABLE parties (
    id                   bigint      GENERATED ALWAYS AS IDENTITY PRIMARY KEY,  -- legacy S_SSN
    tenant_id            bigint      NOT NULL REFERENCES tenants(id),
    card_number          integer     NOT NULL,                    -- legacy S_Card — the business key
    party_type           party_type  NOT NULL DEFAULT 'natural_person',
    first_name           text        NOT NULL,                    -- legacy S_Name
    last_name            text,                                    -- legacy S_Famil
    father_name          text,                                    -- legacy S_Father (persons only)
    id_card_number       text,                                    -- legacy S_IDNO
    birth_date           text,                                    -- legacy S_BDate, Jalali string
    birth_place          text,                                    -- legacy S_BPlace
    id_issue_date        text,                                    -- legacy S_SDate, Jalali string
    id_issue_place       text,                                    -- legacy S_SPlace
    national_id          text,                                    -- legacy S_CodeMelli
    postal_code          text,                                    -- legacy S_CodePosti
    registration_number  text,                                    -- legacy S_CodeSabt
    address              text,                                    -- legacy S_Address
    mobile               text,                                    -- legacy S_Mobile
    tax_status           party_tax_status NOT NULL DEFAULT 'not_specified',  -- legacy S_MaliatState
    is_locked            boolean     NOT NULL DEFAULT false,      -- legacy S_Lock
    created_at            timestamptz NOT NULL DEFAULT now(),
    updated_at            timestamptz,
    created_by            bigint      REFERENCES users(id),
    updated_by            bigint      REFERENCES users(id),
    CONSTRAINT parties_card_number_key UNIQUE (tenant_id, card_number),
    CONSTRAINT parties_first_name_nonblank CHECK (length(btrim(first_name)) > 0)  -- [AS-IS] V1
);

-- V2/V3 (07-03.md): surname required for both kinds, father's name required for persons only —
-- expressed as a CHECK rather than NOT NULL since the requirement is conditional on party_type.
ALTER TABLE parties ADD CONSTRAINT parties_last_name_required
    CHECK (last_name IS NOT NULL AND length(btrim(last_name)) > 0);
ALTER TABLE parties ADD CONSTRAINT parties_father_name_required_for_persons
    CHECK (party_type = 'legal_entity' OR (father_name IS NOT NULL AND length(btrim(father_name)) > 0));

CREATE INDEX parties_tenant_idx ON parties (tenant_id);
CREATE INDEX parties_name_idx ON parties (tenant_id, last_name, first_name);

-- V5/V6 fixed: unique national_id only where present — the legacy's blank-ID hole (§12-Q10, "the
-- first person saved with a blank S_CodeMelli makes every subsequent blank-ID create fail") is not
-- reproduced.
CREATE UNIQUE INDEX parties_national_id_key
    ON parties (tenant_id, national_id) WHERE national_id IS NOT NULL AND btrim(national_id) <> '';

ALTER TABLE parties ENABLE ROW LEVEL SECURITY;
ALTER TABLE parties FORCE ROW LEVEL SECURITY;
CREATE POLICY parties_tenant_isolation ON parties
    USING (tenant_id = current_setting('app.tenant_id')::bigint)
    WITH CHECK (tenant_id = current_setting('app.tenant_id')::bigint);

-- FK pending since 2.1's accounts migration ("party_id (nullable, filled in Phase 3)").
ALTER TABLE accounts ADD CONSTRAINT accounts_party_id_fkey FOREIGN KEY (party_id) REFERENCES parties(id);

-- ---------------------------------------------------------------------
-- party_account_config   <- legacy SahamdarConfig   (07-07.md)
-- Global lookup shape, but tenant-scoped since the chart of accounts (and therefore which Kol/Moein
-- codes are meaningful) is per-tenant (07-07.md §7.1, docs/phase-3-parties.md's Build bullet).
-- ---------------------------------------------------------------------
CREATE TABLE party_account_config (
    id                    bigint  GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    tenant_id             bigint  NOT NULL REFERENCES tenants(id),
    control_kol_code      integer NOT NULL,           -- legacy SC_K
    control_moein_code    integer NOT NULL,           -- legacy SC_M
    -- legacy SC_T: 0 => party card occupies Tafsil-1; >0 => Tafsil-1 is fixed to this code and the
    -- party card occupies Tafsil-2. Kept as the legacy's own 0-sentinel (not NULL) so the natural-key
    -- UNIQUE constraint below works without NULL's not-distinct-from-itself surprise.
    fixed_tafsil1_code    integer NOT NULL DEFAULT 0,
    name                  text    NOT NULL,            -- legacy SC_Name
    for_person            boolean NOT NULL DEFAULT false,  -- legacy SC_1
    for_legal_entity      boolean NOT NULL DEFAULT false,  -- legacy SC_2
    offered_by_default    boolean NOT NULL DEFAULT true,   -- legacy SC_Add
    counts_toward_balance boolean NOT NULL DEFAULT true,   -- legacy SC_Rem
    created_at            timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT party_account_config_name_nonblank CHECK (length(btrim(name)) > 0),
    CONSTRAINT party_account_config_unique
        UNIQUE (tenant_id, control_kol_code, control_moein_code, fixed_tafsil1_code)
);

CREATE INDEX party_account_config_tenant_idx ON party_account_config (tenant_id);

ALTER TABLE party_account_config ENABLE ROW LEVEL SECURITY;
ALTER TABLE party_account_config FORCE ROW LEVEL SECURITY;
CREATE POLICY party_account_config_tenant_isolation ON party_account_config
    USING (tenant_id = current_setting('app.tenant_id')::bigint)
    WITH CHECK (tenant_id = current_setting('app.tenant_id')::bigint);
