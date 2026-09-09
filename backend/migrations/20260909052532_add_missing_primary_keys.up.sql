-- A table with neither a PRIMARY KEY nor an explicit REPLICA IDENTITY makes
-- PostgreSQL reject UPDATE and DELETE on it under logical replication, which is
-- what low-downtime major-version upgrades and CDC pipelines run on. These six had
-- neither; `metrics` is the seventh and lands one migration later.
--
-- The surrogate cannot be swapped for a natural key: every unique index on all six
-- is PARTIAL -- split on `script_hash IS NULL`, or on which of script/flow/app a
-- deployment row describes -- and a partial index cannot back a primary key. The
-- partial uniques stay as they are; they are what the ON CONFLICT clauses infer.
--
-- Each ALTER rewrites its table under ACCESS EXCLUSIVE, holding it unavailable for
-- the length of the rewrite. These six share one because they were together under
-- 50 MB on the largest instance this was measured on, `deployment_metadata` being
-- almost all of it; `metrics` was two orders of magnitude larger and is separated,
-- since Postgres holds every lock a transaction takes until that transaction commits.
--
-- An instance that cannot afford the lock during startup can set REPLICA IDENTITY
-- FULL on these tables instead, which unblocks replication by itself, and apply
-- these ALTERs in a maintenance window; they are idempotent so the migration then
-- finds its work already done.

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
