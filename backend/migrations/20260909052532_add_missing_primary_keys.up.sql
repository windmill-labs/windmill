-- These six had neither a PRIMARY KEY nor an explicit REPLICA IDENTITY, which makes
-- PostgreSQL reject UPDATE and DELETE on them under logical replication. `metrics`
-- is the seventh and lands one migration later.
--
-- The surrogate cannot be swapped for a natural key: every unique index on all six
-- is PARTIAL -- split on `script_hash IS NULL`, or on which of script/flow/app a
-- deployment row describes -- and a partial index cannot back a primary key. The
-- partial uniques stay; they are what the ON CONFLICT clauses infer.
--
-- Each ALTER rewrites its table under ACCESS EXCLUSIVE and holds it unavailable for
-- the rewrite. These six share a transaction because they were together under 50 MB
-- where this was measured; `metrics` was two orders of magnitude larger and is kept
-- out, since Postgres holds a transaction's locks until it commits.
--
-- An instance that cannot afford that lock at startup can set REPLICA IDENTITY FULL
-- on these tables instead, which unblocks replication by itself, and run these
-- idempotent ALTERs in a maintenance window first.

ALTER TABLE deployment_metadata
    ADD COLUMN IF NOT EXISTS id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY;

ALTER TABLE workspace_runnable_dependencies
    ADD COLUMN IF NOT EXISTS id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY;

ALTER TABLE dbt_node
    ADD COLUMN IF NOT EXISTS id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY;

ALTER TABLE dbt_edge
    ADD COLUMN IF NOT EXISTS id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY;

ALTER TABLE dbt_column_edge
    ADD COLUMN IF NOT EXISTS id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY;

ALTER TABLE dbt_graph_snapshot
    ADD COLUMN IF NOT EXISTS id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY;
