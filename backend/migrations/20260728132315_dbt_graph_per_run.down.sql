DELETE FROM dbt_node WHERE job_id <> '00000000-0000-0000-0000-000000000000';
DELETE FROM dbt_edge WHERE job_id <> '00000000-0000-0000-0000-000000000000';
DROP INDEX IF EXISTS idx_dbt_node_job;
ALTER TABLE dbt_node DROP CONSTRAINT IF EXISTS dbt_node_pkey;
ALTER TABLE dbt_node ADD PRIMARY KEY (workspace_id, script_path, script_hash, unique_id);
ALTER TABLE dbt_edge DROP CONSTRAINT IF EXISTS dbt_edge_pkey;
ALTER TABLE dbt_edge
  ADD PRIMARY KEY (workspace_id, script_path, script_hash, parent_unique_id, child_unique_id);
DROP INDEX IF EXISTS idx_dbt_node_run_age;
DROP INDEX IF EXISTS idx_dbt_edge_run_age;
ALTER TABLE dbt_node DROP COLUMN IF EXISTS ingested_at;
ALTER TABLE dbt_edge DROP COLUMN IF EXISTS ingested_at;
ALTER TABLE dbt_node DROP COLUMN IF EXISTS job_id;
ALTER TABLE dbt_edge DROP COLUMN IF EXISTS job_id;
