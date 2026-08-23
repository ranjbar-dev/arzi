-- Step 7.3 (docs/phase-7-hardening-and-cutover.md §7.3 / specs/08-platform-and-security/08-08-
-- backup-restore-new-company-import.md; B20 in specs/11-open-decisions.md): real backup/restore,
-- which the legacy never had (backup existed but restore did not, §8.2's "no unit reads an .ABS
-- archive back" — 08-08.md).
--
-- Authorization judgment call (explicit user decision, 2026-08-23, not guessed): a backup covers
-- the WHOLE shared Postgres instance (this app is multi-tenant/single-database per A3 -- every
-- tenant's data is in the one `pg_dump`), but every existing admin gate (`RequireSuperuser`,
-- 1.3) is scoped to ONE tenant's superuser via the session. Reusing that would let a tenant-A
-- superuser trigger and download a dump containing every OTHER tenant's data too -- a real
-- cross-tenant leak, not a theoretical one, and something no prior spec anticipated (the legacy
-- was single-company). Fixed with a genuinely new, tenant-independent role flag on `users` --
-- `is_platform_admin` -- checked by a new `RequirePlatformAdmin` extractor (api/src/auth/authz.rs),
-- never implied by any tenant's `is_superuser`. Granting the FIRST platform admin is a direct
-- database operation (same "no tenant-provisioning flow exists yet" gap already accepted at 1.1/
-- 2.1/3.1/5.1 for other bootstrap concerns) -- api/src/backup.rs exposes a grant/revoke endpoint
-- for every grant AFTER that, itself gated by RequirePlatformAdmin.

ALTER TABLE users ADD COLUMN is_platform_admin boolean NOT NULL DEFAULT false;

CREATE TYPE backup_status AS ENUM ('running', 'completed', 'failed');

-- Deliberately NOT tenant-scoped, and NO row-level security -- a backup is instance-wide by
-- construction (it is a `pg_dump` of the whole shared database), so a per-tenant RLS policy would
-- be actively misleading here, not a safety net. Every access path is gated at the application
-- layer by RequirePlatformAdmin instead (same "global lookup" exception already used for
-- `journal_sources`/`permissions`, api/migrations/0002_platform_schema.sql's own comment).
CREATE TABLE backups (
    id            bigint  GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    filename      text    NOT NULL UNIQUE,   -- relative path under the backups volume
    status        backup_status NOT NULL DEFAULT 'running',
    size_bytes    bigint,                    -- NULL until completed
    error_message text,                      -- set on failure
    trigger       text    NOT NULL,          -- 'manual' | 'scheduled'
    started_at    timestamptz NOT NULL DEFAULT now(),
    completed_at  timestamptz,
    created_by    bigint  REFERENCES users(id),  -- NULL for scheduled runs (no session)
    CONSTRAINT backups_size_nonneg CHECK (size_bytes IS NULL OR size_bytes >= 0)
);

CREATE INDEX backups_started_at_idx ON backups (started_at DESC);

COMMENT ON TABLE backups IS
  'Tracks every pg_dump run (on-demand or scheduled) and its retention lifecycle. The dump files '
  'themselves live on a Docker-managed named volume mounted only into the api container (durable '
  'across container recreation, never a client-writable path -- the legacy defect this closes, '
  '08-08.md §8.1), not the database. This table is metadata only.';
