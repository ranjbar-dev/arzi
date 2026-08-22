-- Proves the migration harness works end to end. Throwaway — removed once
-- Phase 1 lands the first real table (tenants, fiscal years, org, users).
CREATE TABLE _bootstrap_check (
    id SERIAL PRIMARY KEY,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
