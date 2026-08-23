-- Step 4.1 (docs/phase-4-treasury.md §4.1 / specs/06-treasury/06-01-entity-model.md §1.1-§1.2,
-- 06-02-cheque-state-machine.md): received cheques (legacy DCheck) and their event log (legacy
-- DCheck2), with the B11 state-code ambiguity fixed by construction — a distinct enum value for
-- every real state instead of the legacy's overloaded `S_State=1` (never-deposited AND
-- deposited-then-bounced share one code, §2.1).
--
-- The legacy's dead `TCheck` table (declared, never read or written, §1.0) is not created at all.
-- The three dead endorsement columns (`S_Zssn`/`S_ZCR`/`S_ZName`) are likewise not carried forward
-- — endorsement gets a real model of its own in step 4.3, not these leftover columns.
--
-- Voucher linkage (`voucher_id` on both tables, standing in for legacy `S_Sanad`) is nullable here
-- and deliberately left unset by this step's own transition logic: 4.1's Build bullet is the state
-- machine, 4.2's Build bullet is "wire every transition ... to the Phase 2.5 engine". Posting is
-- 4.2's job, not re-implemented and then replaced here.

CREATE TYPE cheque_status AS ENUM (
    'in_hand',            -- legacy state 1 (never deposited)
    'at_bank',             -- legacy state 2
    'bounced',              -- legacy state 1 overloaded with "in hand" (B11 fix: now distinct)
    'returned_to_issuer',   -- legacy state 4
    'cleared'               -- legacy state 5
    -- 'endorsed_to_third_party' added by step 4.3's own migration.
);

-- ---------------------------------------------------------------------
-- received_cheques   <- legacy DCheck
-- ---------------------------------------------------------------------
CREATE TABLE received_cheques (
    id                          bigint  GENERATED ALWAYS AS IDENTITY PRIMARY KEY,  -- legacy S_SSN
    tenant_id                   bigint  NOT NULL REFERENCES tenants(id),
    fiscal_year_id              bigint  NOT NULL REFERENCES fiscal_years(id),  -- legacy S_COID -- year RECEIVED in, never updated by a transition (§1.1)
    status                      cheque_status NOT NULL DEFAULT 'in_hand',
    cheque_number               text,     -- legacy S_CheckNo -- free text, never validated (§9), matches legacy permissiveness
    received_on                 date    NOT NULL,  -- legacy S_Date
    due_date                    date    NOT NULL,  -- legacy S_DateS
    amount                      bigint  NOT NULL,  -- legacy S_Mab -- unchanged across the whole lifecycle (§2.5)
    description                 text    NOT NULL,  -- legacy S_Desc
    payer_account_id            bigint  NOT NULL REFERENCES accounts(id),  -- legacy S_BesSSN -- must be a leaf, checked in Rust
    notes_receivable_account_id bigint  NOT NULL REFERENCES accounts(id),  -- legacy S_BedSSN
    -- Columns the legacy is missing outright (§1.1's "Missing columns" note) -- smuggled into
    -- free-text S_Desc there, real columns here.
    issuing_bank                text,
    issuing_branch               text,
    issuing_account_number       text,
    drawer_name                  text,
    deposited_at                 date,
    cleared_at                   date,
    bounced_at                   date,
    returned_at                  date,
    voucher_id                   bigint  REFERENCES vouchers(id),  -- legacy S_Sanad -- set by step 4.2
    source_module                smallint NOT NULL DEFAULT 0 REFERENCES journal_sources(id),  -- legacy S_linkPrg
    source_id                    bigint,  -- legacy S_LinkSSN
    created_by                   bigint  REFERENCES users(id),
    created_at                   timestamptz NOT NULL DEFAULT now(),
    updated_at                   timestamptz,
    updated_by                   bigint  REFERENCES users(id),
    CONSTRAINT received_cheques_amount_positive CHECK (amount > 0)
);

CREATE INDEX received_cheques_year_status_idx ON received_cheques (tenant_id, fiscal_year_id, status);
CREATE INDEX received_cheques_due_date_idx    ON received_cheques (tenant_id, due_date);
CREATE INDEX received_cheques_payer_idx       ON received_cheques (tenant_id, payer_account_id);
CREATE INDEX received_cheques_source_idx      ON received_cheques (tenant_id, source_module, source_id)
    WHERE source_id IS NOT NULL;

ALTER TABLE received_cheques ENABLE ROW LEVEL SECURITY;
ALTER TABLE received_cheques FORCE ROW LEVEL SECURITY;
CREATE POLICY received_cheques_tenant_isolation ON received_cheques
    USING (tenant_id = current_setting('app.tenant_id')::bigint)
    WITH CHECK (tenant_id = current_setting('app.tenant_id')::bigint);

COMMENT ON COLUMN received_cheques.status IS
  'B11 fix (11-open-decisions.md): a distinct value for every real state, unlike the legacy''s '
  'S_State which overloads code 1 for both never-deposited and deposited-then-bounced, and never '
  'actually reaches its own declared code 3.';

-- ---------------------------------------------------------------------
-- received_cheque_events   <- legacy DCheck2
-- ---------------------------------------------------------------------
CREATE TABLE received_cheque_events (
    id                  bigint  GENERATED ALWAYS AS IDENTITY PRIMARY KEY,  -- legacy S_SSN
    tenant_id            bigint  NOT NULL REFERENCES tenants(id),
    received_cheque_id   bigint  NOT NULL REFERENCES received_cheques(id) ON DELETE CASCADE,  -- legacy S_Link, given a real FK (legacy has none)
    fiscal_year_id        bigint  NOT NULL REFERENCES fiscal_years(id),  -- legacy S_COID -- of the EVENT, may differ from the cheque's own
    resulting_status       cheque_status NOT NULL,  -- legacy S_State -- the state AFTER this event (B10 fix: always agrees with the master row, see api/src/received_cheques.rs)
    event_date              date    NOT NULL,  -- legacy S_Date
    amount                   bigint  NOT NULL,  -- legacy S_Mab -- always a copy of the cheque's own amount
    debit_account_id         bigint  REFERENCES accounts(id),  -- legacy S_BedSSN
    credit_account_id        bigint  REFERENCES accounts(id),  -- legacy S_BesSSN
    description               text,   -- legacy S_Desc
    voucher_id                bigint  REFERENCES vouchers(id),  -- legacy S_Sanad -- set by step 4.2
    created_by                bigint  REFERENCES users(id),
    created_at                 timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX received_cheque_events_cheque_idx ON received_cheque_events (tenant_id, received_cheque_id, id);

ALTER TABLE received_cheque_events ENABLE ROW LEVEL SECURITY;
ALTER TABLE received_cheque_events FORCE ROW LEVEL SECURITY;
CREATE POLICY received_cheque_events_tenant_isolation ON received_cheque_events
    USING (tenant_id = current_setting('app.tenant_id')::bigint)
    WITH CHECK (tenant_id = current_setting('app.tenant_id')::bigint);

COMMENT ON TABLE received_cheque_events IS
  'Legacy DCheck2. Unlike the legacy, a row IS written for the initial receipt (§2.0''s "the '
  'history of a cheque always starts at its second event" is not reproduced) -- every transition, '
  'including T1, appends one row here.';
