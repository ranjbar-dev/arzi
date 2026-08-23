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

TLS terminates in **host nginx**, not a container — see `deploy/nginx/arzi.conf` for the site
config and its own header comment for the install/certbot steps. `docker-compose.prod.yml`
publishes `web` to `127.0.0.1:${WEB_PORT}` only (not `0.0.0.0`), so nginx on the same host can
reach it but nothing else can bypass nginx to reach it directly. `db` still has no published host
port in prod. `web`'s own `next.config.ts` rewrite already proxies `/api/v1/*` to `api`
server-to-server, so the browser never needs a direct route to either `api` or `db`.

(An earlier version of this stack ran Caddy in a container for automatic HTTPS. Switched to host
nginx because the target server already runs nginx for other sites — one ingress on the box, not
two fighting over 80/443.)

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
5. **TLS live**: with the stack up and nginx configured per `deploy/nginx/arzi.conf` (certbot already
   run) → `curl https://ranjbar.dev/` redirects to `/login`; `curl http://ranjbar.dev/` gets a
   redirect to the HTTPS URL (certbot's nginx plugin adds this automatically).
