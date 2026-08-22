-- Step 2.1 (docs/phase-2-accounting-core.md §2.1 / specs/02-data-model/02-11-c-ddl-parties-and-
-- accounts.md §11.3): the chart of accounts. Legacy Sarfasl — a single flat table holding all four
-- hierarchy levels (Kol/Moein/Tafsil1/Tafsil2), encoded positionally in four integer segments, NOT
-- as an adjacency list (specs/03-accounting-core/03-01-a-account-hierarchy-model.md §1.2-§1.3).
--
-- Two structural proposals from 02-13-a.md §13.6 (`parent_id`/`level` columns) are NOT adopted —
-- docs/phase-2-accounting-core.md §2.1's Build bullet already resolved this: keep the positional
-- segments as the only representation, derive `level` as a generated column instead. §13.7's
-- `party_id` link IS adopted now per the same Build bullet ("party_id (nullable, filled in Phase
-- 3)") — no FK yet since `parties` doesn't exist until step 3.1; that step must add it, same as
-- this migration adds `system_accounts`'s FK now that `accounts` exists (1.1's migration left it
-- pending for exactly this reason).

CREATE TABLE accounts (
    id                   bigint  GENERATED ALWAYS AS IDENTITY PRIMARY KEY,  -- legacy S_SSN
    tenant_id            bigint  NOT NULL REFERENCES tenants(id),
    general_ledger_code  integer NOT NULL DEFAULT 0,  -- legacy S_Ko   کل        (Kol)
    subsidiary_code      integer NOT NULL DEFAULT 0,  -- legacy S_Mo   معین      0 => node IS a Kol
    analytic1_code       integer NOT NULL DEFAULT 0,  -- legacy S_Ta1  تفصیل ۱   0 => node is a Moein
    analytic2_code       integer NOT NULL DEFAULT 0,  -- legacy S_Ta2  تفصیل ۲   0 => node is a Tafsil1
    name                 text    NOT NULL,            -- legacy S_Name
    -- Denormalised child count, application-maintained (not a legacy port of the buggy correlated-
    -- subquery recompute — 03-01-b.md §1.6 — kept incrementally correct by every mutation instead).
    child_count          integer NOT NULL DEFAULT 0,  -- legacy S_Child; 0 => leaf => postable
    is_locked            boolean NOT NULL DEFAULT false,  -- legacy S_Lock; checked hierarchically
    -- Both derived (specs' C3 "derive, don't store" ruling) rather than app-maintained flags.
    level smallint GENERATED ALWAYS AS (
        CASE WHEN subsidiary_code = 0 THEN 1
             WHEN analytic1_code  = 0 THEN 2
             WHEN analytic2_code  = 0 THEN 3
             ELSE 4 END
    ) STORED,
    -- legacy S_Active — resolved (02-11-c.md line 136-137): Active_Set sets it iff the node is a
    -- leaf AND below Kol level (a bare Kol is never "active" in the legacy's sense).
    is_active boolean GENERATED ALWAYS AS (child_count = 0 AND subsidiary_code > 0) STORED,
    party_id             bigint,  -- [NEW] 02-13-a.md §13.7 — FK added by step 3.1 once `parties` exists
    -- Counterparty attributes — a customer/supplier IS a leaf node (01-glossary.md §6b), no
    -- separate counterparty table.
    address              text,
    phone                text,
    fax                  text,
    registration_number  text,
    economic_code        text,
    postal_code          text,
    national_id          text,
    created_at           timestamptz NOT NULL DEFAULT now(),
    updated_at           timestamptz,
    created_by           bigint REFERENCES users(id),
    updated_by           bigint REFERENCES users(id),
    CONSTRAINT accounts_natural_key  -- [AS-IS] — the legacy has a real composite PK here (02-11-c.md line 170)
        UNIQUE (tenant_id, general_ledger_code, subsidiary_code, analytic1_code, analytic2_code),
    CONSTRAINT accounts_name_nonblank CHECK (length(btrim(name)) > 0),
    CONSTRAINT accounts_segments_nonneg
        CHECK (general_ledger_code >= 0 AND subsidiary_code >= 0
               AND analytic1_code >= 0 AND analytic2_code >= 0),
    CONSTRAINT accounts_child_count_nonneg CHECK (child_count >= 0),
    -- The hierarchy is positional: a deeper segment may not be set unless every shallower one is.
    CONSTRAINT accounts_segment_hierarchy
        CHECK ( (subsidiary_code = 0 AND analytic1_code = 0 AND analytic2_code = 0)
             OR (subsidiary_code > 0 AND analytic1_code = 0 AND analytic2_code = 0)
             OR (subsidiary_code > 0 AND analytic1_code > 0 AND analytic2_code = 0)
             OR (subsidiary_code > 0 AND analytic1_code > 0 AND analytic2_code > 0) )
);

CREATE INDEX accounts_gl_idx   ON accounts (tenant_id, general_ledger_code, subsidiary_code);
CREATE INDEX accounts_name_idx ON accounts (tenant_id, name);
CREATE INDEX accounts_leaf_idx ON accounts (tenant_id, id) WHERE child_count = 0;
-- Parent-lookup index for the display-format renderer (walks up to 3 ancestor rows per node,
-- since there's no parent_id — see the module doc comment above).
CREATE INDEX accounts_parent_lookup_idx
    ON accounts (tenant_id, general_ledger_code, subsidiary_code, analytic1_code);

ALTER TABLE accounts ENABLE ROW LEVEL SECURITY;
ALTER TABLE accounts FORCE ROW LEVEL SECURITY;
CREATE POLICY accounts_tenant_isolation ON accounts
    USING (tenant_id = current_setting('app.tenant_id')::bigint)
    WITH CHECK (tenant_id = current_setting('app.tenant_id')::bigint);

-- Pending since 1.1's migration (system_accounts predates accounts).
ALTER TABLE system_accounts
    ADD CONSTRAINT system_accounts_account_id_fkey
    FOREIGN KEY (account_id) REFERENCES accounts(id);

COMMENT ON TABLE accounts IS
  'Legacy Sarfasl. NOT year-scoped (chart is global per tenant, specs 03-01-a.md #1.7 / A6), but IS '
  'tenant-scoped. Fixed 4-level hierarchy Kol -> Moein -> Tafsil1 -> Tafsil2 encoded positionally in '
  'four integers, not an adjacency list -- a node is a leaf, and therefore postable, when '
  'child_count = 0.';
COMMENT ON CONSTRAINT accounts_segment_hierarchy ON accounts IS
  '[NEW] Formalises the implicit rule that the segments fill left to right.';
