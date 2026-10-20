-- Serves get_input_history with include_non_root in windmill-api-inputs: a runnable's runs as a
-- flow step. Every other runnable_path index on v2_job is partial on parent_job IS NULL.
-- runnable_path IS NOT NULL keeps inline flow steps, which no path lookup can match, from
-- paying an index write on every insert.
-- Dropped before it is built because the OVERRIDDEN_MIGRATIONS rewrite in
-- windmill-api/src/db.rs runs both CONCURRENTLY: an interrupted concurrent build leaves an
-- invalid index that IF NOT EXISTS would skip rebuilding.
DROP INDEX IF EXISTS ix_v2_job_child_by_path;

CREATE INDEX IF NOT EXISTS ix_v2_job_child_by_path
    ON v2_job (workspace_id, runnable_path, created_at DESC)
    WHERE parent_job IS NOT NULL AND runnable_path IS NOT NULL;
