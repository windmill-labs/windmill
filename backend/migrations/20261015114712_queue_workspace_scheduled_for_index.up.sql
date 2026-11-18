-- Serves push_scheduled_job's already-exists check in windmill-queue/src/schedule.rs, which
-- looks up the queued jobs of one workspace at one occurrence. Without it that check scans the
-- whole queue on every schedule push.
-- Dropped before it is built because the OVERRIDDEN_MIGRATIONS rewrite in
-- windmill-api/src/db.rs runs both CONCURRENTLY: an interrupted concurrent build leaves an
-- invalid index that IF NOT EXISTS would skip rebuilding.
DROP INDEX IF EXISTS v2_job_queue_workspace_scheduled_for;

CREATE INDEX IF NOT EXISTS v2_job_queue_workspace_scheduled_for
    ON v2_job_queue (workspace_id, scheduled_for);
