#!/usr/bin/env bash
# Step 7.3 (docs/phase-7-hardening-and-cutover.md §7.3): the real restore path api/src/backup.rs's
# own doc comment explains is deliberately NOT an HTTP endpoint — restoring a whole-instance dump
# over the live, shared, multi-tenant database would overwrite every tenant's current data from one
# snapshot while the app is running. This is that operation instead: explicit, run by whoever has
# infra access, defaulting to a SCRATCH database so a restore is exercised (and can be diffed
# against the live database) without ever touching production by accident.
#
# Usage:
#   scripts/restore-backup.sh <dump-file-inside-the-backups-volume-or-local-path> [target-db-name]
#
# Examples:
#   # Restore the newest backup into a scratch database for verification (the default target):
#   scripts/restore-backup.sh arzi-20260823T120000.000Z.dump
#
#   # Real disaster recovery — restore into a fresh database, then point APP config at it:
#   scripts/restore-backup.sh arzi-20260823T120000.000Z.dump arzi_recovered
#
# Must run where `pg_restore` is on PATH and the target Postgres server is reachable via the
# DATABASE_URL family of env vars (PGHOST/PGPORT/PGUSER/PGPASSWORD, or pass a full connection
# string via TARGET_CONN). Reads the dump from inside the `api` container's /backups volume by
# default (docker compose exec) — set DUMP_SOURCE=local to read a path on the machine running this
# script instead.

set -euo pipefail

DUMP_FILE="${1:?usage: restore-backup.sh <dump-file> [target-db-name]}"
TARGET_DB="${2:-scratch_restore_test}"
DUMP_SOURCE="${DUMP_SOURCE:-docker}"

if [ "$DUMP_SOURCE" = "docker" ]; then
    LOCAL_COPY="$(mktemp)"
    echo "Copying $DUMP_FILE out of the api container's backups volume..."
    docker compose cp "api:/backups/$DUMP_FILE" "$LOCAL_COPY"
    DUMP_PATH="$LOCAL_COPY"
else
    DUMP_PATH="$DUMP_FILE"
fi

echo "Creating target database '$TARGET_DB' (dropped first if it already exists)..."
docker compose exec -T db psql -U "${POSTGRES_USER:-arzi}" -d postgres -c "DROP DATABASE IF EXISTS $TARGET_DB;"
docker compose exec -T db psql -U "${POSTGRES_USER:-arzi}" -d postgres -c "CREATE DATABASE $TARGET_DB;"

echo "Restoring into '$TARGET_DB'..."
docker compose cp "$DUMP_PATH" "db:/tmp/restore.dump"
docker compose exec -T db pg_restore -U "${POSTGRES_USER:-arzi}" -d "$TARGET_DB" --no-owner --no-privileges /tmp/restore.dump
docker compose exec -T db rm -f /tmp/restore.dump

if [ "$DUMP_SOURCE" = "docker" ]; then
    rm -f "$LOCAL_COPY"
fi

echo "Restore complete. Sanity check:"
docker compose exec -T db psql -U "${POSTGRES_USER:-arzi}" -d "$TARGET_DB" -c \
    "SELECT (SELECT count(*) FROM tenants) AS tenants, (SELECT count(*) FROM vouchers) AS vouchers, (SELECT count(*) FROM accounts) AS accounts;"
echo "Compare the counts above against the source database (docker compose exec db psql -U \$POSTGRES_USER -d \$POSTGRES_DB -c \"...\") to confirm the restore matches."
