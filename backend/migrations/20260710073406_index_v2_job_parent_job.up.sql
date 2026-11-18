-- Partial index for listing a run's child jobs (flow steps, loop iterations,
-- native-retry attempts, schedule handlers) via the `parent_job = ?` filter on
-- /jobs/list and /jobs/completed/list. Without it, Postgres walks the whole
-- workspace (workspace_id, created_at) timeline filtering row-by-row for the
-- parent. Children of one parent are few, so (parent_job, created_at DESC)
-- returns them directly and serves both ASC and DESC orderings.
-- Partial on parent_job IS NOT NULL keeps it small (root jobs are the majority).
-- Created CONCURRENTLY via the OVERRIDDEN_MIGRATIONS rewrite in windmill-api/src/db.rs.
CREATE INDEX IF NOT EXISTS ix_v2_job_parent_job
    ON v2_job (parent_job, created_at DESC)
    WHERE parent_job IS NOT NULL;
