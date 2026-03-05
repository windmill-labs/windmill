#!/bin/bash
set -euo pipefail

# ── Nix profile ──────────────────────────────────────────────────────────────
export PATH="/root/.nix-profile/bin:/nix/var/nix/profiles/default/bin:$PATH"
export CARGO_HOME=/tmp/.cargo

# ── Browser env (from Nix sandbox profile) ───────────────────────────────────
if command -v sandbox-env >/dev/null 2>&1; then
    eval "$(sandbox-env)"
fi

# ── Start PostgreSQL as current user (no root/su needed) ─────────────────────
PGDATA=/tmp/pgdata
mkdir -p "$PGDATA"

if [ ! -f "$PGDATA/PG_VERSION" ]; then
    initdb -D "$PGDATA" --auth=trust
fi
pg_ctl -D "$PGDATA" -l /tmp/pg.log start -o "-k /tmp"

# Create postgres role and windmill database (idempotent)
psql -h /tmp -d postgres -c "CREATE ROLE postgres SUPERUSER LOGIN" 2>/dev/null || true
createdb -h /tmp windmill 2>/dev/null || true

# ── Run migrations if present ─────────────────────────────────────────────────
if [ -d "$PWD/backend/migrations" ]; then
    DATABASE_URL="postgres://postgres@localhost/windmill?host=/tmp" \
        sqlx migrate run --source "$PWD/backend/migrations" 2>/dev/null || true
fi

# ── Install frontend deps if present ─────────────────────────────────────────
if [ -d "$PWD/frontend" ]; then
    (cd "$PWD/frontend" && npm install && npm run generate-backend-client) 2>/dev/null || true
fi

exec "$@"
