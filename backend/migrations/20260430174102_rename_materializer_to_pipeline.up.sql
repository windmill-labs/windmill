-- Rename the pipeline-membership marker from `materializer` to `pipeline`
-- to better reflect that membership is broader than materialization
-- (test/notify/cleanup scripts are members too without producing assets).
-- The change is text-only — `auto_kind` is a varchar column, not an enum,
-- so this is just a value swap plus an index rebuild.

UPDATE script SET auto_kind = 'pipeline' WHERE auto_kind = 'materializer';

-- Rebuild the partial index that backs the folder picker and graph-scope
-- query. The index name and predicate both move to the new keyword.
DROP INDEX IF EXISTS idx_script_materializer_path;
CREATE INDEX IF NOT EXISTS idx_script_pipeline_path
    ON script (workspace_id, path text_pattern_ops)
    WHERE auto_kind = 'pipeline' AND archived = false AND deleted = false;
