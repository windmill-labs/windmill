#!/usr/bin/env bash
# Configures the local PostgreSQL for logical replication trigger tests.
#
# Prerequisites: wal_level=logical must be set (requires PG restart).
# Check with:  SHOW wal_level;
#
# Usage:
#   ./tests/fixtures/start_postgres_replication.sh          # setup
#   ./tests/fixtures/start_postgres_replication.sh stop     # teardown
set -euo pipefail

PGURL="${DATABASE_URL:-postgres://postgres:changeme@localhost:5432/windmill}"

if [[ "${1:-}" == "stop" ]]; then
    psql "$PGURL" <<'SQL'
SELECT pg_drop_replication_slot('test_e2e_slot')
    WHERE EXISTS (SELECT 1 FROM pg_replication_slots WHERE slot_name = 'test_e2e_slot');
DROP PUBLICATION IF EXISTS test_e2e_pub;
DROP TABLE IF EXISTS test_trigger_table;
SQL
    echo "Postgres replication teardown complete"
    exit 0
fi

# Check wal_level
WAL_LEVEL=$(psql "$PGURL" -tAc "SHOW wal_level;")
if [[ "$WAL_LEVEL" != "logical" ]]; then
    echo "ERROR: wal_level is '$WAL_LEVEL', must be 'logical'"
    echo ""
    echo "Fix with:"
    echo "  psql \"$PGURL\" -c \"ALTER SYSTEM SET wal_level = logical;\""
    echo "  # then restart PostgreSQL"
    exit 1
fi

psql "$PGURL" <<'SQL'
CREATE TABLE IF NOT EXISTS test_trigger_table (id serial PRIMARY KEY, data text);

DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_publication WHERE pubname = 'test_e2e_pub') THEN
        CREATE PUBLICATION test_e2e_pub FOR TABLE test_trigger_table;
    END IF;
END $$;

SELECT pg_create_logical_replication_slot('test_e2e_slot', 'pgoutput')
    WHERE NOT EXISTS (SELECT 1 FROM pg_replication_slots WHERE slot_name = 'test_e2e_slot');
SQL

echo "Postgres logical replication ready (publication=test_e2e_pub, slot=test_e2e_slot)"
echo ""
echo "Run the test:"
echo "  cargo test --test trigger_e2e test_postgres_e2e --features postgres_trigger -- --ignored --nocapture"
