-- Step 4.4 (docs/phase-4-treasury.md §4.4 / specs/06-treasury/06-06-deposit-slips-fish.md,
-- 06-07-petty-cash-tankhah.md): the two flat (no-lifecycle) treasury documents. Both post through
-- the Phase 2.5 engine, same as 4.2's cheques -- real transactional all-or-nothing posting, fixing
-- the legacy's "separate transactions in the same batch" hazard (06-08.md §8.5 defect 6)
-- structurally rather than by convention.
--
-- journal_sources ids 25 (deposit_slip) and 41 (petty_cash) already exist (seeded by 2.3's
-- migration 0006) -- these match the legacy M_Id values (25, 41) exactly, no new rows needed.

CREATE TYPE deposit_channel AS ENUM ('pos_terminal', 'cash_slip', 'card_to_card', 'wire_transfer');

-- ---------------------------------------------------------------------
-- deposit_slips   <- legacy DFish
-- ---------------------------------------------------------------------
CREATE TABLE deposit_slips (
    id                bigint  GENERATED ALWAYS AS IDENTITY PRIMARY KEY,  -- legacy S_SSN
    tenant_id          bigint  NOT NULL REFERENCES tenants(id),
    fiscal_year_id      bigint  NOT NULL REFERENCES fiscal_years(id),  -- legacy S_COID
    slip_number          text,    -- legacy S_FishNo -- free text, never validated (§6.3), matches legacy permissiveness
    slip_date              date    NOT NULL,  -- legacy S_Date
    amount                   bigint  NOT NULL,  -- legacy S_Mab
    description               text,   -- legacy S_Desc -- NOT required non-blank, unlike the cheque screen (§1.3)
    payer_account_id           bigint  NOT NULL REFERENCES accounts(id),  -- legacy S_BesSSN -- the party who paid, credited
    bank_account_id              bigint  NOT NULL REFERENCES accounts(id),  -- legacy S_BankSSN -- the bank/cash account, debited
    channel                        deposit_channel NOT NULL,  -- legacy S_State -- descriptive only, §6.2
    voucher_id                      bigint  REFERENCES vouchers(id),  -- legacy S_Sanad
    source_module                    smallint NOT NULL DEFAULT 0 REFERENCES journal_sources(id),  -- legacy S_LinkPRG
    source_id                         bigint,  -- legacy S_LinkSSN
    created_by                         bigint  REFERENCES users(id),
    created_at                          timestamptz NOT NULL DEFAULT now(),
    updated_at                           timestamptz,
    updated_by                            bigint  REFERENCES users(id),
    CONSTRAINT deposit_slips_amount_positive CHECK (amount > 0)
);

CREATE INDEX deposit_slips_year_date_idx ON deposit_slips (tenant_id, fiscal_year_id, slip_date);

ALTER TABLE deposit_slips ENABLE ROW LEVEL SECURITY;
ALTER TABLE deposit_slips FORCE ROW LEVEL SECURITY;
CREATE POLICY deposit_slips_tenant_isolation ON deposit_slips
    USING (tenant_id = current_setting('app.tenant_id')::bigint)
    WITH CHECK (tenant_id = current_setting('app.tenant_id')::bigint);

COMMENT ON COLUMN deposit_slips.description IS
  'B-defect fix (06-08.md §8.5 defect 2): the legacy attaches its two narration strings to the '
  'wrong sides ("by <payer>" lands on the bank-debit line, "to <bank>" on the payer-credit line). '
  'The rebuild composes ONE correct narration and applies it to both voucher lines, structurally '
  'preventing the swap rather than fixing two separately-built strings.';

-- ---------------------------------------------------------------------
-- petty_cash_claims   <- legacy TankhahMaster
-- ---------------------------------------------------------------------
CREATE TABLE petty_cash_claims (
    id                    bigint  GENERATED ALWAYS AS IDENTITY PRIMARY KEY,  -- legacy TM_SSN
    tenant_id              bigint  NOT NULL REFERENCES tenants(id),
    fiscal_year_id          bigint  NOT NULL REFERENCES fiscal_years(id),  -- legacy TM_Coid
    claim_number              text,    -- legacy TM_No -- free text
    claim_date                  date    NOT NULL,  -- legacy TM_Date
    description                   text,   -- legacy TM_Desc
    custodian_account_id           bigint  NOT NULL REFERENCES accounts(id),  -- legacy TM_Code -- credited for the total
    -- Denormalised, application-maintained (C3) -- NOT entered, computed from the lines (§7.2),
    -- unlike the legacy's Set_Sum out-of-transaction recompute.
    total_amount                     bigint  NOT NULL DEFAULT 0,
    line_count                        integer NOT NULL DEFAULT 0,
    voucher_id                         bigint  REFERENCES vouchers(id),  -- legacy TM_Sanad
    created_by                          bigint  REFERENCES users(id),
    created_at                           timestamptz NOT NULL DEFAULT now(),
    updated_at                            timestamptz,
    updated_by                             bigint  REFERENCES users(id),
    CONSTRAINT petty_cash_claims_total_nonneg CHECK (total_amount >= 0),
    CONSTRAINT petty_cash_claims_line_count_nonneg CHECK (line_count >= 0)
);

CREATE INDEX petty_cash_claims_year_date_idx ON petty_cash_claims (tenant_id, fiscal_year_id, claim_date);

ALTER TABLE petty_cash_claims ENABLE ROW LEVEL SECURITY;
ALTER TABLE petty_cash_claims FORCE ROW LEVEL SECURITY;
CREATE POLICY petty_cash_claims_tenant_isolation ON petty_cash_claims
    USING (tenant_id = current_setting('app.tenant_id')::bigint)
    WITH CHECK (tenant_id = current_setting('app.tenant_id')::bigint);

COMMENT ON COLUMN petty_cash_claims.description IS
  'B-defect fix (06-07.md §7.3): the legacy''s credit-line narration always says "... تعداد N نفر" '
  '("count N PERSONS"), copy-paste residue from the issued-cheque batch screen where it was '
  'correct -- expense lines are not people. The rebuild composes a narration that actually '
  'describes expense lines.';

-- ---------------------------------------------------------------------
-- petty_cash_claim_lines   <- legacy TankhahDetail
-- ---------------------------------------------------------------------
CREATE TABLE petty_cash_claim_lines (
    id                  bigint  GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    tenant_id            bigint  NOT NULL REFERENCES tenants(id),
    claim_id              bigint  NOT NULL REFERENCES petty_cash_claims(id) ON DELETE CASCADE,  -- legacy TD_TMSSN
    expense_account_id      bigint  NOT NULL REFERENCES accounts(id),  -- legacy TD_Bed -- debited
    amount                    bigint  NOT NULL,  -- legacy TD_Mab
    description                text,   -- legacy TD_Desc -- becomes the voucher line's narration verbatim
    created_at                   timestamptz NOT NULL DEFAULT now(),
    created_by                    bigint  REFERENCES users(id),
    CONSTRAINT petty_cash_claim_lines_amount_positive CHECK (amount > 0)
);

CREATE INDEX petty_cash_claim_lines_claim_idx ON petty_cash_claim_lines (tenant_id, claim_id);

ALTER TABLE petty_cash_claim_lines ENABLE ROW LEVEL SECURITY;
ALTER TABLE petty_cash_claim_lines FORCE ROW LEVEL SECURITY;
CREATE POLICY petty_cash_claim_lines_tenant_isolation ON petty_cash_claim_lines
    USING (tenant_id = current_setting('app.tenant_id')::bigint)
    WITH CHECK (tenant_id = current_setting('app.tenant_id')::bigint);
