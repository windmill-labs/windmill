-- These five had neither a PRIMARY KEY nor an explicit REPLICA IDENTITY, which makes
-- PostgreSQL reject UPDATE and DELETE on them under logical replication.
-- `deployment_metadata` and `metrics` are the other two, one migration each after this.
--
-- The surrogate cannot be swapped for a natural key: every unique index on all five
-- is PARTIAL -- split on `script_hash IS NULL`, or on which of flow/app a dependency
-- row describes -- and a partial index cannot back a primary key. The partial uniques
-- stay; they are what the ON CONFLICT clauses infer.
--
-- Each ALTER rewrites its table under ACCESS EXCLUSIVE and holds it unavailable for
-- the rewrite. These five share a transaction because each is bounded by what a
-- workspace holds rather than by how long it has run, so none can grow into the one
-- that locks the rest; a transaction holds all its locks until it commits.
--
-- An instance that cannot afford that lock at startup can set REPLICA IDENTITY FULL
-- on these tables instead, which unblocks replication by itself, and run these
-- idempotent ALTERs in a maintenance window first.

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
