-- Step 1.4 (docs/phase-1-platform-and-auth.md §1.4 / specs/08-05-audit-trail-
-- change-log.md §5.3): a real, append-only audit log. The legacy has none —
-- no login history, no permission-change history, no deletion history, no
-- before-values anywhere (only the latest editor id survives on a handful of
-- denormalised stamp columns). This table is the general-purpose replacement,
-- written by a Rust helper (api/src/audit.rs) called inside the same
-- transaction as the mutation it records — not a per-table Postgres trigger
-- (specs/10-target-architecture.md §2.6's stance on keeping logic in Rust).
--
-- Two kinds of row share this table:
--   * Row mutations on a real business table: table_name = the table
--     ('users', and every domain table from later phases), record_id = that
--     row's id, action = 'insert'/'update'/'delete'.
--   * Security events with no single business-table row to diff (a login
--     attempt, a permission-set replace): table_name = 'auth_events',
--     record_id = the affected user's id (or NULL when no user resolves, e.g.
--     a login attempt against a nonexistent username), action = a semantic
--     name ('login_succeeded', 'login_failed', 'password_changed',
--     'permission_granted', 'permission_revoked').

CREATE TABLE audit_log (
    id          bigint      GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    tenant_id   bigint      NOT NULL REFERENCES tenants(id),
    table_name  text        NOT NULL,
    record_id   text,
    action      text        NOT NULL,
    changed_by  bigint      REFERENCES users(id),
    changed_at  timestamptz NOT NULL DEFAULT now(),
    old_values  jsonb,
    new_values  jsonb
);

CREATE INDEX audit_log_tenant_idx ON audit_log (tenant_id, changed_at DESC);
CREATE INDEX audit_log_table_record_idx ON audit_log (tenant_id, table_name, record_id);

ALTER TABLE audit_log ENABLE ROW LEVEL SECURITY;
ALTER TABLE audit_log FORCE ROW LEVEL SECURITY;
CREATE POLICY audit_log_tenant_isolation ON audit_log
    USING (tenant_id = current_setting('app.tenant_id')::bigint)
    WITH CHECK (tenant_id = current_setting('app.tenant_id')::bigint);

COMMENT ON TABLE audit_log IS
  'Append-only. Nothing update/deletes rows here -- no route in the API ever '
  'issues UPDATE or DELETE against this table. Tenant-scoped like every other '
  'business table (08-05-audit-trail-change-log.md #5.3); unlike sessions/'
  'tenants/permissions there is no chicken-and-egg problem, since every write '
  'happens from inside an already-authenticated, already tenant-scoped '
  'request.';
