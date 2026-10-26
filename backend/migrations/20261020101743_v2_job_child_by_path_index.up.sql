-- Serves get_input_history in windmill-api-inputs when it lists a runnable's runs inside other
-- runs. Every other runnable_path index on v2_job is partial on parent_job IS NULL. That listing
-- reads only script and flow runs, so the kind predicate spares inline steps, loop and branch
-- containers and agent steps, most of the child jobs, an index write on every insert.
-- Dropped before it is built because the OVERRIDDEN_MIGRATIONS rewrite in
-- windmill-api/src/db.rs runs both CONCURRENTLY: an interrupted concurrent build leaves an
-- invalid index that IF NOT EXISTS would skip rebuilding.
DROP INDEX IF EXISTS ix_v2_job_child_by_path;

CREATE INDEX IF NOT EXISTS ix_v2_job_child_by_path
    ON v2_job (workspace_id, runnable_path, created_at DESC)
    WHERE parent_job IS NOT NULL AND kind IN ('script', 'flow');
