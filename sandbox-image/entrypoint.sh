#!/bin/bash
set -euo pipefail

# ── Start PostgreSQL ──────────────────────────────────────────────────────────
PGDATA=/tmp/pgdata
mkdir -p "$PGDATA"
chown postgres:postgres "$PGDATA"

if [ ! -f "$PGDATA/PG_VERSION" ]; then
    su - postgres -c "/usr/lib/postgresql/15/bin/initdb -D $PGDATA --auth=trust"
fi
su - postgres -c "/usr/lib/postgresql/15/bin/pg_ctl -D $PGDATA -l /tmp/pg.log start -o '-k /tmp'"
su - postgres -c "psql -h /tmp -c 'CREATE ROLE root SUPERUSER LOGIN'" 2>/dev/null || true
su - postgres -c "createdb -h /tmp windmill" 2>/dev/null || true

# ── Run migrations if present ─────────────────────────────────────────────────
if [ -d "$PWD/backend/migrations" ]; then
    DATABASE_URL="postgres://root@localhost/windmill?host=/tmp" \
        sqlx migrate run --source "$PWD/backend/migrations" 2>/dev/null || true
fi

# ── Install frontend deps if present ─────────────────────────────────────────
if [ -d "$PWD/frontend" ]; then
    (cd "$PWD/frontend" && npm install && npm run generate-backend-client) 2>/dev/null || true
fi

exec "$@"
