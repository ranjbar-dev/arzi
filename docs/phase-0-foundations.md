# Phase 0 — Foundations

No domain logic yet. Gets an empty system running end to end (browser → API → database) so every
later phase is additive.

---

## 0.1 Repo scaffold & Docker Compose

**Goal:** `docker compose up` brings up an API, a web app and a database that can all reach each
other, with nothing to see yet but a health check.

**Build**

- Cargo workspace: `api/` crate (`axum` + `tokio`), empty `GET /health` returning `200 {"status":"ok"}`.
- Next.js app in `web/` (App Router, TypeScript, Tailwind CSS 4 configured), one placeholder page.
- `docker-compose.yml`: `api`, `web`, `db` (`postgres:17`, named volume, health check), per
  `10-target-architecture.md` §5. Config via environment variables only — no secrets baked into the
  image.
- `.env.example` documenting every variable the compose file expects.

**Spec refs:** `10-target-architecture.md` §5.

**Manual test**

1. `docker compose up --build`.
2. `curl http://localhost:<api-port>/health` → `200`, JSON body `{"status":"ok"}`.
3. Open `http://localhost:<web-port>` in a browser → placeholder page renders, no console errors.
4. `docker compose down && docker compose up` again → same result (volume persists, nothing crashes
   on restart).

**Done when:** all three containers report healthy and step 2–3 pass from a clean `docker compose up`.

---

## 0.2 Database bootstrap & migration tooling

**Goal:** schema changes are managed through versioned migrations, not manual `psql` edits, from the
first table onward.

**Build**

- `sqlx::migrate!` wired into `api` startup (runs pending migrations on boot in dev; explicit
  `sqlx migrate run` step for prod, not automatic — per `10-target-architecture.md` §2.1 no magic at
  startup for production).
- First migration: empty, just proves the harness works (`CREATE TABLE _bootstrap_check (...)` or
  similar throwaway, removable once Phase 1 lands real tables).
- Pooled connection (`sqlx::PgPool`) shared via `axum` state.
- Transaction helper stub: a function that opens a transaction and is the single place that will
  later issue `SET LOCAL app.tenant_id = $1` (per `10-target-architecture.md` §2.4) — implemented
  now as a passthrough since tenancy doesn't exist until Phase 1, but the call site exists so Phase 1
  only fills it in rather than threading it through every handler retroactively.

**Spec refs:** `10-target-architecture.md` §2.1, §2.4.

**Manual test**

1. `sqlx migrate run` (or let `api` run it on boot) against the `db` container.
2. `psql` into `db`, `\dt` → migration tracking table plus the bootstrap table exist.
3. Restart `api` → no errors, migration does not re-run (already applied).
4. Add a trivial second migration, confirm it applies on next run and `\dt` shows the change.

**Done when:** migrations apply cleanly from empty, are idempotent on restart, and the connection
pool is reachable from an `axum` handler (extend `/health` to do a `SELECT 1` and report DB status).
