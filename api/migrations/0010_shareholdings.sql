-- Step 3.3 (docs/phase-3-parties.md §3.3 / specs/07-parties-and-shareholders/07-05-shareholder-
-- equity-profit-distribution.md, A4 in 11-open-decisions.md — "DECIDED: absorb into rebuild"):
-- real shareholder-equity logic. The legacy has NONE of this (07-05.md's exhaustive "derivation of
-- absence" — zero hits for profit/loss/capital/percent in a business sense) — this is genuinely new
-- logic, not a port, per A4's ruling and this step's Build bullet. Not integrated with the external
-- Saham.Dbo product (\\pesteh\SahamData\) — a clean new subsystem that happens to sit next to the
-- party register.

CREATE TABLE shareholdings (
    id            bigint  GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    tenant_id     bigint  NOT NULL REFERENCES tenants(id),
    party_id      bigint  NOT NULL REFERENCES parties(id),  -- a shareholder IS a party, kept as an
                                                              -- independent fact from their current-
                                                              -- account balance (3.1/3.2) — no shared row.
    share_count   bigint  NOT NULL,
    nominal_value bigint  NOT NULL DEFAULT 0,  -- rials, per-share
    join_date     date    NOT NULL,
    exit_date     date,  -- NULL => still an active holding
    created_at    timestamptz NOT NULL DEFAULT now(),
    updated_at    timestamptz,
    created_by    bigint  REFERENCES users(id),
    updated_by    bigint  REFERENCES users(id),
    CONSTRAINT shareholdings_share_count_positive CHECK (share_count > 0),
    CONSTRAINT shareholdings_nominal_value_nonneg CHECK (nominal_value >= 0),
    CONSTRAINT shareholdings_exit_after_join CHECK (exit_date IS NULL OR exit_date >= join_date)
);

-- `ownership_percentage` is NOT a stored/generated column (specs' C3 "derive, don't store" ruling,
-- same as accounts.level/is_active) — it depends on an aggregate over every OTHER row for the same
-- tenant, which Postgres generated columns cannot express. Computed in api/src/shareholdings.rs.

CREATE INDEX shareholdings_tenant_idx ON shareholdings (tenant_id);
CREATE INDEX shareholdings_party_idx  ON shareholdings (tenant_id, party_id);

ALTER TABLE shareholdings ENABLE ROW LEVEL SECURITY;
ALTER TABLE shareholdings FORCE ROW LEVEL SECURITY;
CREATE POLICY shareholdings_tenant_isolation ON shareholdings
    USING (tenant_id = current_setting('app.tenant_id')::bigint)
    WITH CHECK (tenant_id = current_setting('app.tenant_id')::bigint);
