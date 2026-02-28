-- Partial index for fast failure/canceled filtering on the runs page.
-- When failures are sparse (<1%) this avoids scanning millions of successful jobs.
-- The query orders by completed_at DESC (switched from created_at when success=false),
-- so this index provides both filtering and ordering in a single scan.
CREATE INDEX IF NOT EXISTS ix_v2_job_completed_failure_workspace
    ON v2_job_completed (workspace_id, completed_at DESC)
    WHERE status IN ('failure', 'canceled');
