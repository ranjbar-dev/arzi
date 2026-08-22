#!/bin/sh
# Runs once, on first cluster init (postgres image convention:
# docker-entrypoint-initdb.d). Creates the role the API connects as for
# everyday queries — separate from POSTGRES_USER, which owns the tables and
# runs migrations. Per specs/10-target-architecture.md §4 / step 1.1: the
# API's role must NOT own these tables and must NOT have BYPASSRLS, or RLS
# is decorative.
set -e

psql -v ON_ERROR_STOP=1 --username "$POSTGRES_USER" --dbname "$POSTGRES_DB" <<-SQL
    CREATE ROLE "$APP_DB_USER" LOGIN PASSWORD '$APP_DB_PASSWORD'
        NOSUPERUSER NOCREATEDB NOCREATEROLE NOBYPASSRLS;
    GRANT CONNECT ON DATABASE "$POSTGRES_DB" TO "$APP_DB_USER";
    GRANT USAGE ON SCHEMA public TO "$APP_DB_USER";
    -- Applies to tables/sequences $POSTGRES_USER creates from here on (i.e.
    -- everything the migrations add) — no per-migration GRANT needed.
    ALTER DEFAULT PRIVILEGES FOR ROLE "$POSTGRES_USER" IN SCHEMA public
        GRANT SELECT, INSERT, UPDATE, DELETE ON TABLES TO "$APP_DB_USER";
    ALTER DEFAULT PRIVILEGES FOR ROLE "$POSTGRES_USER" IN SCHEMA public
        GRANT USAGE, SELECT ON SEQUENCES TO "$APP_DB_USER";
SQL
