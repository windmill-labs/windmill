-- Consolidate live index migrations into a regular SQL migration.
-- All statements are idempotent (IF EXISTS / IF NOT EXISTS).

-- === DROP obsolete indexes ===
DROP INDEX IF EXISTS ix_completed_job_workspace_id_created_at;
DROP INDEX IF EXISTS ix_completed_job_workspace_id_created_at_new;
DROP INDEX IF EXISTS index_completed_job_on_schedule_path;
DROP INDEX IF EXISTS concurrency_limit_stats_queue;
DROP INDEX IF EXISTS root_job_index;
DROP INDEX IF EXISTS index_completed_on_created;
DROP INDEX IF EXISTS root_job_index_by_path_2;
DROP INDEX IF EXISTS ix_completed_job_workspace_id_created_at_new_2;
DROP INDEX IF EXISTS ix_completed_job_workspace_id_started_at_new;
DROP INDEX IF EXISTS root_job_index_by_path;
DROP INDEX IF EXISTS labeled_jobs_on_jobs;
DROP INDEX IF EXISTS ix_job_workspace_id_created_at_new_6;
DROP INDEX IF EXISTS ix_job_workspace_id_created_at_new_7;
DROP INDEX IF EXISTS queue_sort;
DROP INDEX IF EXISTS queue_sort_2;
DROP INDEX IF EXISTS log_file_hostname_log_ts_idx;
DROP INDEX IF EXISTS ix_completed_job_workspace_id_started_at_new_2;
DROP INDEX IF EXISTS ix_job_created_at;
DROP INDEX IF EXISTS ix_v2_job_root_by_path;

-- === CREATE indexes ===
CREATE INDEX IF NOT EXISTS ix_job_workspace_id_created_at_new_3
    ON v2_job (workspace_id, created_at DESC);

CREATE INDEX IF NOT EXISTS ix_job_workspace_id_created_at_new_8
    ON v2_job (workspace_id, created_at DESC)
    WHERE kind IN ('deploymentcallback') AND parent_job IS NULL;

CREATE INDEX IF NOT EXISTS ix_job_workspace_id_created_at_new_9
    ON v2_job (workspace_id, created_at DESC)
    WHERE kind IN ('dependencies', 'flowdependencies', 'appdependencies') AND parent_job IS NULL;

CREATE INDEX IF NOT EXISTS ix_job_workspace_id_created_at_new_5
    ON v2_job (workspace_id, created_at DESC)
    WHERE kind IN ('preview', 'flowpreview') AND parent_job IS NULL;

CREATE INDEX IF NOT EXISTS labeled_jobs_on_jobs
    ON v2_job_completed USING GIN ((result -> 'wm_labels'))
    WHERE result ? 'wm_labels';

CREATE INDEX IF NOT EXISTS ix_v2_job_labels
    ON v2_job USING GIN (labels)
    WHERE labels IS NOT NULL;

ALTER TABLE v2_job ENABLE ROW LEVEL SECURITY;

CREATE INDEX IF NOT EXISTS ix_v2_job_workspace_id_created_at
    ON v2_job (workspace_id, created_at DESC)
    WHERE kind IN ('script', 'flow', 'singlestepflow') AND parent_job IS NULL;

CREATE INDEX IF NOT EXISTS queue_sort_v2
    ON v2_job_queue (priority DESC NULLS LAST, scheduled_for, tag)
    WHERE running = false;

CREATE INDEX IF NOT EXISTS ix_audit_timestamps
    ON audit (timestamp DESC);

CREATE INDEX IF NOT EXISTS ix_job_completed_completed_at
    ON v2_job_completed (completed_at DESC);

CREATE INDEX IF NOT EXISTS alerts_by_workspace
    ON alerts (workspace_id);

CREATE INDEX IF NOT EXISTS v2_job_queue_suspend
    ON v2_job_queue (workspace_id, suspend)
    WHERE suspend > 0;

CREATE INDEX IF NOT EXISTS idx_audit_recent_login_activities
    ON audit (timestamp, username)
    WHERE operation IN ('users.login', 'oauth.login', 'users.token.refresh');

CREATE INDEX IF NOT EXISTS script_not_archived
    ON script (workspace_id, path, created_at DESC)
    WHERE archived = false;

CREATE INDEX IF NOT EXISTS ix_job_workspace_id_completed_at_all
    ON v2_job_completed (workspace_id, completed_at DESC);

CREATE INDEX IF NOT EXISTS idx_job_v2_job_root_by_path_2
    ON v2_job (workspace_id, runnable_path)
    WHERE parent_job IS NULL;

CREATE INDEX IF NOT EXISTS ix_job_root_job_index_by_path_2
    ON v2_job (workspace_id, runnable_path, created_at DESC)
    WHERE parent_job IS NULL;
