-- Add down migration script here
-- Remove performance indexes

DROP INDEX IF EXISTS queue_sort_v2;
DROP INDEX IF EXISTS queue_suspended;
DROP INDEX IF EXISTS root_queue_index_by_path;
DROP INDEX IF EXISTS v2_job_queue_suspend;
DROP INDEX IF EXISTS ix_completed_job_workspace_id_started_at_new_2;
DROP INDEX IF EXISTS ix_job_completed_completed_at;
DROP INDEX IF EXISTS labeled_jobs_on_jobs;
DROP INDEX IF EXISTS ix_v2_job_workspace_id_created_at;
DROP INDEX IF EXISTS ix_v2_job_labels;
DROP INDEX IF EXISTS outstanding_wait_time_job_id_idx;
DROP INDEX IF EXISTS job_perms_job_id_idx;
DROP INDEX IF EXISTS job_perms_workspace_created_idx;