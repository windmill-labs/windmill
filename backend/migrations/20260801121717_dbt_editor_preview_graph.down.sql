-- The editor graphs go first: they are exactly the rows the restored NOT NULL
-- would reject, and they are throwaway by construction.
DELETE FROM dbt_node WHERE script_hash IS NULL;
DELETE FROM dbt_edge WHERE script_hash IS NULL;
DELETE FROM dbt_graph_snapshot WHERE script_hash IS NULL;

DROP INDEX IF EXISTS idx_dbt_graph_snapshot_editor_path;
ALTER TABLE dbt_graph_snapshot DROP COLUMN IF EXISTS permissioned_as;

DROP INDEX IF EXISTS dbt_node_editor_key;
DROP INDEX IF EXISTS dbt_node_versioned_key;
ALTER TABLE dbt_node ALTER COLUMN script_hash SET NOT NULL;
ALTER TABLE dbt_node ADD CONSTRAINT dbt_node_pkey
  PRIMARY KEY (workspace_id, script_path, script_hash, job_id, unique_id);

DROP INDEX IF EXISTS dbt_edge_editor_key;
DROP INDEX IF EXISTS dbt_edge_versioned_key;
ALTER TABLE dbt_edge ALTER COLUMN script_hash SET NOT NULL;
ALTER TABLE dbt_edge ADD CONSTRAINT dbt_edge_pkey
  PRIMARY KEY (workspace_id, script_path, script_hash, job_id, parent_unique_id, child_unique_id);

DROP INDEX IF EXISTS dbt_graph_snapshot_editor_key;
DROP INDEX IF EXISTS dbt_graph_snapshot_versioned_key;
ALTER TABLE dbt_graph_snapshot ALTER COLUMN script_hash SET NOT NULL;
ALTER TABLE dbt_graph_snapshot ADD CONSTRAINT dbt_graph_snapshot_pkey
  PRIMARY KEY (workspace_id, script_path, script_hash, job_id);
