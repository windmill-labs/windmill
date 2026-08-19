DROP TABLE IF EXISTS dbt_run_progress;
DROP TABLE IF EXISTS dbt_run_state;
DROP TABLE IF EXISTS dbt_graph_snapshot;
DROP TABLE IF EXISTS dbt_edge;
DROP TABLE IF EXISTS dbt_node;

ALTER TABLE workspace_settings DROP COLUMN IF EXISTS dbt_warehouses;
