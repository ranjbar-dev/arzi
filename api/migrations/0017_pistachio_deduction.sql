-- Step 5.6 (docs/phase-5-inventory.md §5.6 / specs/05-inventory/05-08-a-pesteh-pistachio-
-- specialisation.md §8.0-§8.2): the deduction formula finally reachable in the ordinary
-- purchase-invoice flow (B19) — the legacy wrote the correct arithmetic (`PestehD_U.pas`) but
-- shipped it on a permanently hidden panel behind a Save button with no `OnClick` handler at all
-- (§8.0.1). One row per pistachio-grade purchase line, linked by real surrogate FK (never the
-- legacy's "one integer plays three roles" convention §8.1.1 already fixed at 5.1 for
-- pistachio_grades/items/accounts — this table is the fourth explicit link in that same spirit).

CREATE TABLE pistachio_deduction_details (
    id                     bigint  GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    tenant_id              bigint  NOT NULL REFERENCES tenants(id),
    document_line_id       bigint  NOT NULL REFERENCES inventory_document_lines(id),
    bale_count             integer NOT NULL,                          -- Adl — tare driver, mandatory (§8.2.1)
    tare_allowance_kg      numeric(4,2) NOT NULL,                     -- AdlV -> 0.1 / 0.2 / 1.0 kg per bale
    gross_weight_kg        numeric(10,1) NOT NULL,                    -- BascV — mandatory (§8.2.1)
    moisture_pct           numeric(5,2)  NOT NULL DEFAULT 0,          -- Rot, % of gross
    blank_pct              numeric(5,2)  NOT NULL DEFAULT 0,          -- Pook, % of gross
    other_deductions_kg    numeric(10,1) NOT NULL DEFAULT 0,          -- Sayer — entered in kg, not a percentage
    tare_deduction_kg      numeric(10,3) NOT NULL,                    -- derived, stored for display (§8.2.2)
    moisture_deduction_kg  numeric(10,3) NOT NULL,                    -- derived
    blank_deduction_kg     numeric(10,3) NOT NULL,                    -- derived
    total_deduction_kg     numeric(10,3) NOT NULL,                    -- derived — may exceed gross_weight_kg,
                                                                        -- displayed as-is (§8.2.2: only net_weight floors)
    net_weight_kg          numeric(10,3) NOT NULL,                    -- derived, floored at 0 — this is also the
                                                                        -- linked line's `quantity`
    created_at             timestamptz NOT NULL DEFAULT now(),
    created_by             bigint  REFERENCES users(id),
    CONSTRAINT pistachio_deduction_details_line_key UNIQUE (document_line_id),
    CONSTRAINT pistachio_deduction_details_bale_count_positive CHECK (bale_count > 0),        -- §8.2.3 manual test 3
    CONSTRAINT pistachio_deduction_details_gross_weight_positive CHECK (gross_weight_kg > 0),  -- real, not cosmetic
    CONSTRAINT pistachio_deduction_details_tare_allowance_valid
        CHECK (tare_allowance_kg IN (0.1, 0.2, 1.0))                  -- AdlV's three combo values only
);

-- §8.2.2's deduction floor: net_weight legitimately floors to 0 when total deductions exceed
-- gross weight, and the legacy explicitly allows saving that line with no block at all ("no
-- error, no warning, no block", §8.2.2/manual test #2's own "not silently saved unnoticed either"
-- framing implies it CAN be saved). 5.2's own `quantity > 0` CHECK (a plain judgment call at the
-- time, not spec-mandated) is relaxed here to accommodate this legitimate zero-quantity case,
-- discovered only once this step tried to exercise it.
ALTER TABLE inventory_document_lines DROP CONSTRAINT inventory_document_lines_quantity_positive;
ALTER TABLE inventory_document_lines ADD CONSTRAINT inventory_document_lines_quantity_nonnegative
    CHECK (quantity >= 0);

CREATE INDEX pistachio_deduction_details_tenant_idx ON pistachio_deduction_details (tenant_id);

ALTER TABLE pistachio_deduction_details ENABLE ROW LEVEL SECURITY;
ALTER TABLE pistachio_deduction_details FORCE ROW LEVEL SECURITY;
CREATE POLICY pistachio_deduction_details_tenant_isolation ON pistachio_deduction_details
    USING (tenant_id = current_setting('app.tenant_id')::bigint)
    WITH CHECK (tenant_id = current_setting('app.tenant_id')::bigint);
