ALTER TABLE dbt_node ADD COLUMN IF NOT EXISTS graph_digest TEXT;
DROP TABLE IF EXISTS dbt_graph_snapshot;
