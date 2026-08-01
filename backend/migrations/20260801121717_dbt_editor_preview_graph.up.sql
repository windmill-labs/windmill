-- A graph parsed from the EDITOR's buffer, which names no deployed version.
--
-- The two provenances that existed keyed on one: a version's own graph (the
-- zero-UUID `job_id`) and a run's snapshot of a version. A buffer refresh is
-- neither. It cannot borrow the deployed version's hash — the buffer differs
-- from it, which is the point, and a project being written has no deployed
-- version at all — so `script_hash` becomes NULL and the graph is keyed to the
-- preview job that parsed it, readable only back through that job and never
-- through the path.
--
-- The foreign key to `script` is left exactly as written: it is MATCH SIMPLE,
-- so a NULL in either column satisfies it. A versioned row still dies with its
-- version and a version-less one is simply outside its reach.
--
-- Two partial unique indexes rather than one with NULLS NOT DISTINCT, which
-- needs Postgres 15: a versioned graph is keyed by its version, a buffer parse
-- by its job alone (a preview job id is unique on its own).
ALTER TABLE dbt_node DROP CONSTRAINT dbt_node_pkey;
ALTER TABLE dbt_node ALTER COLUMN script_hash DROP NOT NULL;
CREATE UNIQUE INDEX IF NOT EXISTS dbt_node_versioned_key
  ON dbt_node (workspace_id, script_path, script_hash, job_id, unique_id)
  WHERE script_hash IS NOT NULL;
CREATE UNIQUE INDEX IF NOT EXISTS dbt_node_editor_key
  ON dbt_node (workspace_id, job_id, unique_id)
  WHERE script_hash IS NULL;

ALTER TABLE dbt_edge DROP CONSTRAINT dbt_edge_pkey;
ALTER TABLE dbt_edge ALTER COLUMN script_hash DROP NOT NULL;
CREATE UNIQUE INDEX IF NOT EXISTS dbt_edge_versioned_key
  ON dbt_edge (workspace_id, script_path, script_hash, job_id, parent_unique_id, child_unique_id)
  WHERE script_hash IS NOT NULL;
CREATE UNIQUE INDEX IF NOT EXISTS dbt_edge_editor_key
  ON dbt_edge (workspace_id, job_id, parent_unique_id, child_unique_id)
  WHERE script_hash IS NULL;

ALTER TABLE dbt_graph_snapshot DROP CONSTRAINT dbt_graph_snapshot_pkey;
ALTER TABLE dbt_graph_snapshot ALTER COLUMN script_hash DROP NOT NULL;
CREATE UNIQUE INDEX IF NOT EXISTS dbt_graph_snapshot_versioned_key
  ON dbt_graph_snapshot (workspace_id, script_path, script_hash, job_id)
  WHERE script_hash IS NOT NULL;
CREATE UNIQUE INDEX IF NOT EXISTS dbt_graph_snapshot_editor_key
  ON dbt_graph_snapshot (workspace_id, job_id)
  WHERE script_hash IS NULL;

-- An editor graph is bounded per path rather than by age: the newest few are
-- kept and the rest dropped as each refresh lands, so this is the index that
-- ordering reads. The instance-wide age sweep every dbt run already performs
-- (`job_id <> DEPLOYED_GRAPH`) still catches a path refreshed once and left.
CREATE INDEX IF NOT EXISTS idx_dbt_graph_snapshot_editor_path
  ON dbt_graph_snapshot (workspace_id, script_path, ingested_at)
  WHERE script_hash IS NULL;
