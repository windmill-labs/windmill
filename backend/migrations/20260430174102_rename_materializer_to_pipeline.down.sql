-- Revert: rename pipeline-membership marker back to materializer.
DROP INDEX IF EXISTS idx_script_pipeline_path;
CREATE INDEX IF NOT EXISTS idx_script_materializer_path
    ON script (workspace_id, path text_pattern_ops)
    WHERE auto_kind = 'materializer' AND archived = false AND deleted = false;

UPDATE script SET auto_kind = 'materializer' WHERE auto_kind = 'pipeline';
