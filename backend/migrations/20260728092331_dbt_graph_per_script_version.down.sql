ALTER TABLE dbt_node DROP CONSTRAINT IF EXISTS dbt_node_script_fkey;
ALTER TABLE dbt_edge DROP CONSTRAINT IF EXISTS dbt_edge_script_fkey;
ALTER TABLE dbt_node DROP CONSTRAINT dbt_node_pkey;
ALTER TABLE dbt_edge DROP CONSTRAINT dbt_edge_pkey;
-- Collapsing back to one graph per path: keep the newest version's rows only,
-- since the old key cannot hold two.
DELETE FROM dbt_node a USING dbt_node b
 WHERE a.workspace_id = b.workspace_id AND a.script_path = b.script_path
   AND a.unique_id = b.unique_id AND a.script_hash < b.script_hash;
DELETE FROM dbt_edge a USING dbt_edge b
 WHERE a.workspace_id = b.workspace_id AND a.script_path = b.script_path
   AND a.parent_unique_id = b.parent_unique_id AND a.child_unique_id = b.child_unique_id
   AND a.script_hash < b.script_hash;
ALTER TABLE dbt_node ADD PRIMARY KEY (workspace_id, script_path, unique_id);
ALTER TABLE dbt_edge ADD PRIMARY KEY (workspace_id, script_path, parent_unique_id, child_unique_id);
ALTER TABLE dbt_node DROP COLUMN IF EXISTS script_hash;
ALTER TABLE dbt_edge DROP COLUMN IF EXISTS script_hash;
