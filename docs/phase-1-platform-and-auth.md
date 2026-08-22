# Phase 1 — Platform, Tenancy & Auth

Everything every other phase depends on: who's logged in, which tenant and fiscal year they're
acting in, what they're allowed to do, and that every mutation leaves a trace. Nothing here is a
port of the legacy mechanism — `08-03`/`08-04`/`08-05` document what existed only to show why none
of it transfers (plaintext passwords, presentation-only checks, no audit trail at all).

---

## 1.1 Core schema: tenants, fiscal years, organisation, users + RLS

**Goal:** the tenancy boundary exists in the database itself, not just in application code.

**Build**

- Migrate the `tenants`, `users`, `permissions`, `user_permissions`, `fiscal_years`, `organization`,
  `account_code_format`, `app_settings`, `system_accounts`, `user_preferences`,
  `settings_audit_log` tables verbatim from `02-11-b-ddl-platform.md` §11.2 — every table gets
  `ENABLE`/`FORCE ROW LEVEL SECURITY` and the `current_setting('app.tenant_id')` policy as shown.
- The Postgres role the API connects as must **not** own these tables and must **not** have
  `BYPASSRLS` (`10-target-architecture.md` §4) — create a separate migration-owner role if needed.
- Seed the `permissions` catalogue (global, not tenant-scoped) from the 85-permission matrix in
  `08-04-authorization.md` §4.4 — same numeric ids (1100–2125), clean English `code`, Persian
  `label_fa`. Skip the known-dead ones (1129, 1130, 1415, 1416, 2119 gap) — do not port orphan grants.
- Extend the transaction helper from step 0.2: it now actually issues
  `SET LOCAL app.tenant_id = $1` as its first statement, sourced only from the authenticated
  session, never from a request header or body field (`10-target-architecture.md` §2.4).

**Spec refs:** `02-11-b-ddl-platform.md` §11.2; `11-open-decisions.md` A3.

**Manual test**

1. Insert two tenants directly via `psql`.
2. As the API's Postgres role, run `SET LOCAL app.tenant_id = '<tenant-1-id>'` then
   `INSERT INTO users (...) VALUES (...)` for both tenants inside one session — confirm the
   tenant-2 insert is rejected by the RLS `WITH CHECK` when `app.tenant_id` is still set to tenant 1.
3. With `app.tenant_id` set to tenant 1, `SELECT * FROM users` → only tenant 1's rows come back,
   even though tenant 2 rows exist in the table.
4. Confirm the API's Postgres role genuinely cannot bypass this: `SELECT rolbypassrls FROM pg_roles
   WHERE rolname = '<api-role>'` → `false`.

**Done when:** cross-tenant reads/writes are blocked by Postgres itself with the API role connected,
not merely by application-layer filtering.

---

## 1.2 Authentication

**Goal:** real login — Argon2id, no plaintext, no enumerable username list, server-side sessions.

**Build**

- `POST /api/v1/auth/login` — body `{ tenantSlug, username, password }`. Resolve `tenant_id` from
  `tenantSlug` **first**, then look up `username` scoped to that tenant, then verify with `argon2`.
  Never search for a username across tenants (`02-11-b.md` §11.2 comment on `users`).
- Generic failure message for wrong tenant/username/password/disabled — do not distinguish which one
  failed (kills the legacy's enumerable dropdown + distinct error messages, `08-03.md` §3.3 steps 3–4).
- No lockout-free retry loop: add a basic rate limit per (tenant, username) pair — the legacy had
  none at all (`08-03.md` §3.4); this is a new, minimal control, not a large feature.
- Sessions: a `sessions` table (`id`, `user_id`, `tenant_id`, `created_at`, `expires_at`,
  `revoked_at`), session id in an `HttpOnly`, `Secure`, `SameSite=Lax` cookie. Login writes a row;
  every authenticated request loads it and rejects if expired/revoked.
- `POST /api/v1/auth/logout` revokes the current session.
- `POST /api/v1/auth/change-password` — require current password (exact match, like the legacy
  `ChangePasswordU.pas`, but hashed) — hash the new one, no length/complexity theatre beyond a
  sane minimum (8 chars), invalidate other sessions on change.
- New users are created with **no usable password** (a null/unset hash that can never verify) and a
  mandatory first-login "set your password" flow — this directly closes the legacy's "new user has
  empty password and can log in immediately" hole (`08-04.md` §4.3).

**Spec refs:** `08-03-authentication.md` (as the "don't do this" reference); `10-target-architecture.md` §2.5.

**Manual test**

1. Create a tenant + user (with a set password) directly via a seed script or `psql` + a one-off
   Argon2id hash.
2. `curl -X POST /api/v1/auth/login` with correct tenant/username/password → `200`, `Set-Cookie`
   session header.
3. Same call with wrong password → generic `401`, same body shape as a wrong username would give.
4. `curl` a protected endpoint with the session cookie → `200`. Without it → `401`.
5. `curl -X POST /api/v1/auth/logout` then retry the protected endpoint with the same cookie → `401`.
6. Try creating a user via the admin flow (step 1.3) and logging in immediately with a blank
   password → must fail; only the "set password" flow can activate the account.

**Done when:** no password is ever stored or logged in plaintext, sessions expire/revoke correctly,
and a freshly created user cannot log in before setting a password.

---

## 1.3 Authorization

**Goal:** every mutating and every sensitive-read endpoint checks a real permission, server-side,
before touching the database — not a UI toggle.

**Build**

- `authz::has_permission(user_id, permission_code) -> bool`, backed by `user_permissions` joined to
  `permissions`, cached per-request (load once at session start, not once-per-check like the legacy's
  ~35-query `IsEnabel` sweep, `08-04.md` §4.2).
- `is_superuser` bypasses the check **within that user's own tenant only** — never across tenants
  (`02-11-b.md` §11.2 `users.is_superuser` comment).
- An `axum` extractor/middleware (`RequirePermission(code)`) applied at the route level — the
  legacy's model of "checked in ~5 places, everywhere else presentation-only" is exactly what must
  not be repeated (`08-04.md` §4.2 "the check is presentation-only").
- Admin endpoints: list users, grant/revoke permissions (transactional replace, not the legacy's
  unwrapped delete-then-reinsert, `08-04.md` §4.2), enable/disable user, create user (goes through
  the "no usable password" path from 1.2).
- Every permission id from the seeded catalogue (1.1) maps to at least one route by the end of every
  later phase — track this as each phase wires its own routes (steps 2.8, 4.x implicitly, 6.7).

**Spec refs:** `08-04-authorization.md` §4.1–§4.4 (matrix + the specific "orphan"/"dead"/"ambiguous"
grants in the closing tables — do not port those ambiguities, resolve each as a clean single mapping).

**Manual test**

1. Create two users in the same tenant: one with a permission granted, one without.
2. Call the corresponding endpoint as each — granted user gets `200`/expected result, ungranted user
   gets `403`, not a filtered/degraded response.
3. Grant then revoke the permission via the admin endpoint, confirm the check flips live (no stale
   cache beyond the current session, or document the session-scoped cache behaviour and confirm a
   re-login picks up the change).
4. As a superuser in tenant A, confirm they can act on tenant A data without explicit grants, and
   confirm (via 1.1's RLS) they still cannot touch tenant B's data at all.

**Done when:** a `curl` call with no session, a wrong session, and a session lacking the permission
all fail distinctly and correctly — and nothing enforces authorization only in the frontend.

---

## 1.4 Audit log

**Goal:** every mutation is traceable — who, what, when, before/after — which the legacy has *zero*
of (`08-05.md` §5.3: no login history, no permission-change history, no deletion history, no
settings-change history, nothing).

**Build**

- `audit_log` table: `id`, `tenant_id`, `table_name`, `record_id`, `action` (`insert`/`update`/
  `delete`), `changed_by`, `changed_at`, `old_values jsonb`, `new_values jsonb`. Append-only, RLS
  tenant-scoped like every other table.
- A single Rust helper called by every domain service on every mutation (not a Postgres trigger per
  table — keeps the logic visible in Rust per `10-target-architecture.md` §2.6's stance on triggers/
  procedures) — call it from inside the same transaction as the mutation it's logging.
- Auth events (`login_succeeded`, `login_failed`, `password_changed`, `permission_granted`,
  `permission_revoked`, `user_created`, `user_disabled`) recorded through the same table using a
  synthetic `table_name` like `auth_events` — this is new, the legacy records none of it (`08-05.md`
  §5.3, first four bullets).
- **Item change-history (A17 — decided "add it"):** inventory-item mutations (Phase 5) write through
  this same `audit_log`, not a bespoke shadow table like the legacy's undocumented `AnbarJens_B`
  trigger. No separate schema object needed for this — it's covered once `audit_log` exists and every
  domain service calls the helper consistently.
- `settings_audit_log` (already created in step 1.1's migration) covers `system_accounts` and the
  other accounting-behaviour-affecting settings specifically, per `02-11-b.md`'s comment on that
  table — keep it distinct from the general `audit_log` since it's the one place §13.19's stricter
  requirement (never miss a change) applies.

**Spec refs:** `08-05-audit-trail-change-log.md` §5.2–§5.3.

**Manual test**

1. Create, update and delete a `user` (or any table wired by this point) through the API.
2. `SELECT * FROM audit_log WHERE table_name = 'users' ORDER BY changed_at` → three rows, correct
   `action`, `old_values`/`new_values` reflecting the real before/after, correct `changed_by`.
3. Attempt a login with a wrong password → an `auth_events` row with `action = 'login_failed'`
   appears, no plaintext password anywhere in it.
4. Grant a permission via 1.3's admin endpoint → a row appears logging the grant, with `changed_by`
   correctly identifying the admin who did it (fixes the legacy's "no history of grants" gap).

**Done when:** for any mutation made through the API in this phase, there is a corresponding
`audit_log` row with enough detail to answer "who changed this, from what, to what, when."

---

## 1.5 Fiscal year management

**Goal:** an explicit, auditable "create fiscal year" and "close fiscal year" action — the legacy has
no screen that ever writes `Base.IsActive` at all (`02-11-b.md` §11.2 comment on `fiscal_years.is_active`).

**Build**

- `POST /api/v1/fiscal-years` — create a new fiscal year row (`start_date`, `end_date`, `year`),
  validated against `fiscal_years_no_overlap`/`fiscal_years_date_order` from the DDL. Chart of
  accounts is **not** copied per year — stays global, matching current (if silent) behaviour (A6).
- `POST /api/v1/fiscal-years/{id}/close` — the **explicit admin action** implementing A5: sets
  `is_active = false`. Does not itself carry forward balances — that's the accounting-core year-end
  flow in step 2.7, which this endpoint's contract must anticipate (`closing_voucher_id` /
  `opening_voucher_id` columns already exist on `fiscal_years` from the 1.1 migration).
- `GET /api/v1/fiscal-years` (list) and a "current fiscal year" concept per user session (which year
  they're acting in right now) — every subsequent domain read/write is implicitly scoped to this,
  same as legacy `CO_ID` scoped everything (`00-overview.md` fact 1).
- Switching the active fiscal year is a real state change with a real Cancel — B21 fix: the legacy
  bug where Cancel on the fiscal-year switcher applied the change anyway (`ChangesU.pas:78`) must not
  be reproduced. Confirm the frontend (1.6) only commits the switch on explicit confirm.

**Spec refs:** `07-01-a/b-company-multi-tenancy-model.md`; A5, A6 in `11-open-decisions.md`.

**Manual test**

1. Create fiscal year 1403 (some date range) via the API.
2. Attempt to create an overlapping fiscal year for the same tenant → rejected by the DB constraint.
3. Create fiscal year 1404 (non-overlapping) → succeeds.
4. Switch the session's active fiscal year to 1404, confirm subsequent calls report 1404 as current.
5. Close fiscal year 1403 → `is_active` flips to `false`; confirm a later phase's posting attempt
   against a closed year is rejected (revisit this assertion once step 2.3 exists).

**Done when:** fiscal years are created and closed only through explicit, auditable actions, and no
code path can silently flip `is_active`.

---

## 1.6 Frontend shell

**Goal:** a Persian, RTL, authenticated Next.js shell to build every later screen inside.

**Build**

- Next.js App Router project, Tailwind CSS 4, `dir="rtl"` at the document root, CSS logical
  properties throughout (`margin-inline-start`, not `margin-left`) per `10-target-architecture.md`
  §3.3.
- **Before building any screen here, install and use the `ui-ux-pro-max` skill** (see
  `00-overview.md`) for the design-system decisions (palette, type pairing, spacing) — apply it once
  to establish the shell's look, then reuse those tokens for every later phase's screens rather than
  re-deriving them per phase.
- i18n scaffold (`fa` locale) — every user-visible string is a translation key from the start, not
  retrofitted; seed the `fa` locale with the Persian captions already captured verbatim throughout
  `specs/`.
- Persian-Indic digit formatting at the presentation layer only (`۰۱۲۳`); store/compute ASCII.
- Login page (calls 1.2), protected route wrapper (redirects to login on `401`), session-aware layout
  showing current tenant/fiscal-year/user.
- Top-level navigation shell matching the six domains from `08-01-the-complete-main-menu-tree.md`
  (accounting, inventory, treasury, parties, reporting, platform/settings) — just the shell and
  routing skeleton; each domain's actual screens are built in their own phase. Keyboard navigation is
  explicitly a new addition here (the legacy ribbon has none) — at minimum, make the nav focusable
  and operable without a mouse from the start rather than bolting it on later.
- Admin screens for 1.3's user/permission endpoints (list users, grant/revoke permissions, create
  user) — this is the direct replacement for the legacy `Admin.pas` grid-of-checkboxes.

**Spec refs:** `10-target-architecture.md` §3.2–§3.3; `08-01-the-complete-main-menu-tree.md`.

**Manual test**

1. Open the app unauthenticated → redirected to login.
2. Log in → land on shell with correct tenant name, active fiscal year, RTL layout, Persian nav
   labels.
3. Tab through the nav using only the keyboard → every top-level item reachable and activatable.
4. As a non-superuser without the admin permission, confirm the admin nav item is absent or disabled
   — then confirm (per 1.3) the admin API route itself also rejects them even if they hit it directly.
5. Create a user and grant a permission through the admin screen, confirm it matches the `audit_log`
   row from step 1.4's manual test.

**Done when:** a real user can log in, see a correctly-shelled Persian RTL app, and manage other
users/permissions entirely through the UI.
