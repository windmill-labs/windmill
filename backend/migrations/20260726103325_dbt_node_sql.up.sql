-- The model's own SQL and where it lives in the repo, so the asset graph can
-- show the transform behind a node instead of only its name and materialization.
ALTER TABLE dbt_node ADD COLUMN IF NOT EXISTS raw_code TEXT;
ALTER TABLE dbt_node ADD COLUMN IF NOT EXISTS original_file_path TEXT;
