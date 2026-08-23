-- Step 4.5 follow-up (docs/phase-4-treasury.md §4.5 / specs/06-treasury/06-01-entity-model.md
-- §1.4-§1.5, 06-11-c-screen-specifications.md §11.9-11.11): issued-cheque payment batches (legacy
-- `CheckMaster`/`CheckDetail`), unblocked from A9 (specs/11-open-decisions.md) by an explicit user
-- decision (2026-08-23): treat `CheckMaster` as a payment batch (header + N payee lines), matching
-- the DDL-shape evidence already on record (header/detail structure, `CM_Count` cached line count)
-- -- NOT confirmed against real data (the reference dump has zero rows and neither table has a
-- primary key), documented as an inferred-not-confirmed call, not a settled fact. Revisit if a
-- populated legacy database ever becomes available.
--
-- `journal_sources` id 26 ('cheque_payment_document') already exists (seeded by 2.3's migration
-- 0006), matching the legacy M_Id exactly -- no new row needed.

-- ---------------------------------------------------------------------
-- cheque_payment_batches   <- legacy CheckMaster
-- ---------------------------------------------------------------------
CREATE TABLE cheque_payment_batches (
    id                bigint  GENERATED ALWAYS AS IDENTITY PRIMARY KEY,  -- legacy CM_SSN
    tenant_id          bigint  NOT NULL REFERENCES tenants(id),
    fiscal_year_id      bigint  NOT NULL REFERENCES fiscal_years(id),  -- legacy CM_Coid
    batch_number          text,    -- legacy CM_No -- free text, not validated
    issue_date               date    NOT NULL,  -- legacy CM_Date
    description                text    NOT NULL,  -- legacy CM_Desc -- non-blank enforced (06-01.md §1.4)
    letter_body                  text,   -- legacy CM_Tittle -- three-line covering-letter body, optional
    bank_account_id                bigint  NOT NULL REFERENCES accounts(id),  -- legacy CM_Code -- credited for the batch total
    -- Denormalised, application-maintained (C3) -- computed from the lines, not entered (legacy
    -- Set_Sum grid-footer recompute, done here in-transaction instead).
    total_amount                     bigint  NOT NULL DEFAULT 0,
    line_count                        integer NOT NULL DEFAULT 0,
    voucher_id                         bigint  REFERENCES vouchers(id),  -- legacy CM_Sanad
    created_by                          bigint  REFERENCES users(id),
    created_at                           timestamptz NOT NULL DEFAULT now(),
    updated_at                            timestamptz,
    updated_by                             bigint  REFERENCES users(id),
    CONSTRAINT cheque_payment_batches_total_nonneg CHECK (total_amount >= 0),
    CONSTRAINT cheque_payment_batches_line_count_nonneg CHECK (line_count >= 0)
);

CREATE INDEX cheque_payment_batches_year_date_idx ON cheque_payment_batches (tenant_id, fiscal_year_id, issue_date);

ALTER TABLE cheque_payment_batches ENABLE ROW LEVEL SECURITY;
ALTER TABLE cheque_payment_batches FORCE ROW LEVEL SECURITY;
CREATE POLICY cheque_payment_batches_tenant_isolation ON cheque_payment_batches
    USING (tenant_id = current_setting('app.tenant_id')::bigint)
    WITH CHECK (tenant_id = current_setting('app.tenant_id')::bigint);

-- ---------------------------------------------------------------------
-- cheque_payment_batch_lines   <- legacy CheckDetail
-- ---------------------------------------------------------------------
CREATE TABLE cheque_payment_batch_lines (
    id                        bigint  GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    tenant_id                  bigint  NOT NULL REFERENCES tenants(id),
    batch_id                    bigint  NOT NULL REFERENCES cheque_payment_batches(id) ON DELETE CASCADE,  -- legacy CD_CMSSN
    payee_account_id              bigint  NOT NULL REFERENCES accounts(id),  -- legacy CD_Bed -- debited
    amount                          bigint  NOT NULL,  -- legacy CD_Mab
    description                       text,   -- legacy CD_Desc -- becomes the voucher line's narration verbatim
    payee_bank_account_number           text,   -- legacy CD_BankNo -- IBAN/account number, free text, never validated (§1.5)
    payee_account_holder_name             text,   -- legacy CD_Jari -- name on the payee's bank account, never used in posting
    created_at                              timestamptz NOT NULL DEFAULT now(),
    created_by                               bigint  REFERENCES users(id),
    CONSTRAINT cheque_payment_batch_lines_amount_positive CHECK (amount > 0)
);

CREATE INDEX cheque_payment_batch_lines_batch_idx ON cheque_payment_batch_lines (tenant_id, batch_id);

ALTER TABLE cheque_payment_batch_lines ENABLE ROW LEVEL SECURITY;
ALTER TABLE cheque_payment_batch_lines FORCE ROW LEVEL SECURITY;
CREATE POLICY cheque_payment_batch_lines_tenant_isolation ON cheque_payment_batch_lines
    USING (tenant_id = current_setting('app.tenant_id')::bigint)
    WITH CHECK (tenant_id = current_setting('app.tenant_id')::bigint);
