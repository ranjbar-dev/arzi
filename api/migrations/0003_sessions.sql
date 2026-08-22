-- Step 1.2 (docs/phase-1-platform-and-auth.md §1.2 / specs/10-target-architecture.md §2.5):
-- server-side session store backing cookie-based login. Replaces the legacy's
-- total absence of sessions (08-03-authentication.md §3.3 set globals on a
-- data module with no expiry or revocation at all).

CREATE TABLE sessions (
    id          text        PRIMARY KEY,
    user_id     bigint      NOT NULL REFERENCES users(id),
    tenant_id   bigint      NOT NULL REFERENCES tenants(id),
    created_at  timestamptz NOT NULL DEFAULT now(),
    expires_at  timestamptz NOT NULL,
    revoked_at  timestamptz
);

CREATE INDEX sessions_user_idx ON sessions (user_id);

COMMENT ON TABLE sessions IS
  'Server-side sessions (specs/10-target-architecture.md §2.5). No RLS, unlike '
  'every tenant-scoped table from 0002: a session is looked up by its own id '
  '(a 256-bit random token, the cookie value) before tenant context is known '
  '-- same chicken-and-egg exception already made for tenants/permissions '
  '(see 0002''s comment on tenants). The unguessable token is the access '
  'boundary here, not RLS.';
