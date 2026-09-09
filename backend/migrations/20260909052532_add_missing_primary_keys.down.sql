-- Dropping the column drops the primary key and the identity sequence with it, and
-- only marks the column dropped in the catalog rather than rewriting the table, so
-- this takes the ACCESS EXCLUSIVE lock but not the time.

ALTER TABLE deployment_metadata DROP COLUMN IF EXISTS id;
ALTER TABLE workspace_runnable_dependencies DROP COLUMN IF EXISTS id;
ALTER TABLE dbt_node DROP COLUMN IF EXISTS id;
ALTER TABLE dbt_edge DROP COLUMN IF EXISTS id;
ALTER TABLE dbt_column_edge DROP COLUMN IF EXISTS id;
ALTER TABLE dbt_graph_snapshot DROP COLUMN IF EXISTS id;
