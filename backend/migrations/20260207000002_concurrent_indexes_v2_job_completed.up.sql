-- v2_job_completed: drop obsolete indexes and create new ones CONCURRENTLY
DROP INDEX CONCURRENTLY IF EXISTS ix_completed_job_workspace_id_created_at;
DROP INDEX CONCURRENTLY IF EXISTS ix_completed_job_workspace_id_created_at_new;
DROP INDEX CONCURRENTLY IF EXISTS index_completed_job_on_schedule_path;
DROP INDEX CONCURRENTLY IF EXISTS index_completed_on_created;
DROP INDEX CONCURRENTLY IF EXISTS ix_completed_job_workspace_id_created_at_new_2;
DROP INDEX CONCURRENTLY IF EXISTS ix_completed_job_workspace_id_started_at_new;
DROP INDEX CONCURRENTLY IF EXISTS ix_completed_job_workspace_id_started_at_new_2;
DROP INDEX CONCURRENTLY IF EXISTS labeled_jobs_on_jobs;

CREATE INDEX CONCURRENTLY IF NOT EXISTS labeled_jobs_on_jobs
    ON v2_job_completed USING GIN ((result -> 'wm_labels'))
    WHERE result ? 'wm_labels';

CREATE INDEX CONCURRENTLY IF NOT EXISTS ix_job_completed_completed_at
    ON v2_job_completed (completed_at DESC);

CREATE INDEX CONCURRENTLY IF NOT EXISTS ix_job_workspace_id_completed_at_all
    ON v2_job_completed (workspace_id, completed_at DESC);
