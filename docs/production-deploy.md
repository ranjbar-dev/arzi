# Production deploy

Step 7.4 (`docs/phase-7-hardening-and-cutover.md §7.4`). `docker-compose.yml` is the base shape
(same file local dev already uses); `docker-compose.prod.yml` is the override — see that file's own
header comment for what it changes and why.

## Build and run

```sh
docker compose -f docker-compose.yml -f docker-compose.prod.yml build
docker compose -f docker-compose.yml -f docker-compose.prod.yml up -d
```

All configuration is environment variables (`.env`, or your secrets manager injecting the same
names) — nothing in either compose file or either Dockerfile hard-codes a credential or connection
string. Copy `.env.example`, replace every `change_me*` value with a real secret, and set
`SESSION_COOKIE_SECURE=true` only once TLS is actually terminated in front of the stack (a plain-HTTP
browser silently drops a `Secure` cookie — 1.6's own finding).

## Migrations

Local dev runs pending migrations automatically on `api` boot (`RUN_MIGRATIONS_ON_BOOT` unset/`1`).
The prod override sets `RUN_MIGRATIONS_ON_BOOT=0` — migrations are an explicit, auditable step, run
via the **"Database migrations"** GitHub Actions workflow (`.github/workflows/migrate.yml`), not
alongside container startup:

1. Add a repository secret `PROD_DATABASE_URL` — a connection string for the **owner** role (the
   same role `api/main.rs`'s own `DATABASE_URL` uses; never the RLS-bound `APP_DATABASE_URL` role,
   which cannot create/alter tables by design).
2. Run the workflow from the Actions tab (`workflow_dispatch` — deliberately manual, not on every
   push; a schema migration against production deserves a deliberate click) before deploying a
   release that depends on a new migration.

The workflow installs `sqlx-cli` and runs `sqlx migrate run` against that secret — the same command
`main.rs`'s boot-time path runs internally, just invoked explicitly and auditable in the Actions log
instead of silently on container start.

## TLS termination

`caddy` (`docker-compose.prod.yml`) fronts `web` on 80/443 with automatic HTTPS — config in the
repo-root `Caddyfile`. Set `SITE_DOMAIN` in `.env` to your real public domain for automatic Let's
Encrypt; left unset, it defaults to `localhost` and Caddy issues a self-signed cert from its own
internal CA instead (works unmodified for local prod-shape testing — confirmed live, see below).
`web`/`db` have no published host ports in prod — `caddy` is the only public ingress; `web`'s own
`next.config.ts` rewrite already proxies `/api/v1/*` to `api` server-to-server, so the browser never
needs a direct route to either `api` or `db`.

## Manual test procedure (this step's own "Done when")

1. **Clean build**: `docker compose -f docker-compose.yml -f docker-compose.prod.yml build
   --no-cache` from a clean checkout → both `api` and `web` images build successfully.
   `docker images | grep arzi` → confirm sizes are reasonable (no build toolchain baked into the
   runtime stage — `api`'s runtime is `debian:trixie-slim` + `postgresql-client-17` only, no `cargo`
   or `rustc`; `web`'s runtime is `node:20-alpine` with only the built `.next` output, no dev
   dependencies).
2. **Full stack up**: `docker compose -f docker-compose.yml -f docker-compose.prod.yml up -d` with
   real (non-`change_me`) env values → `docker compose ps` shows all three healthy.
3. **DB outage recovery**: `docker compose stop db`, then poll `curl http://localhost:$API_PORT/health`
   → `db: unreachable` (never a 200 with a stale/cached "ok"); `docker compose start db` → `/health`
   returns `db: ok` again within its own healthcheck interval, no `api` restart required (the pool
   reconnects on its own — `sqlx::PgPool` handles this natively, nothing bespoke needed here).
4. **Credential grep**: `grep -rniE "password *= *['\"][^$]" api/Dockerfile web/Dockerfile
   docker-compose.yml docker-compose.prod.yml` → matches nothing (every credential is an
   environment-variable *reference*, `${VAR}` or `$VAR`, never a literal value baked into a file
   that gets committed).
5. **TLS live**: `docker compose -f docker-compose.yml -f docker-compose.prod.yml up -d` with real
   env values → `curl -k https://localhost/` (or your real domain, without `-k`) redirects to
   `/login`; `curl http://localhost/` gets a 308 to the HTTPS URL (Caddy's automatic HTTP→HTTPS
   redirect). Confirmed live on this repo's dev host (port 443 was already OS-reserved there, so
   verified via a temporary alternate port mapping — the Caddyfile and compose service themselves
   are unchanged and use the real 80/443 in the file as committed).
