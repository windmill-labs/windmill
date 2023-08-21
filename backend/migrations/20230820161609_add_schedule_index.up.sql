-- Add up migration script here
CREATE INDEX IF NOT EXISTS scheduled_root_job ON completed_job (workspace_id, schedule_path, started_at) WHERE parent_job is NULL;
CREATE INDEX IF NOT EXISTS root_job_index_by_path ON completed_job (workspace_id, script_path, job_kind, created_at) WHERE parent_job is NULL;
CREATE INDEX IF NOT EXISTS root_job_index ON completed_job (workspace_id, job_kind, created_at) WHERE parent_job is NULL;
DROP INDEX IF EXISTS index_completed_on_script_hash;
DROP INDEX IF EXISTS index_completed_on_script_path;
DROP INDEX IF EXISTS index_completed_on_schedule_path;
DROP INDEX IF EXISTS index_completed_on_workspace_id;
DROP INDEX IF EXISTS index_completed_on_created_at;

DROP INDEX IF EXISTS index_queue_on_script_path;
DROP INDEX IF EXISTS index_queue_on_script_hash;
DROP INDEX IF EXISTS index_queue_on_workspace_id;
DROP INDEX IF EXISTS index_queue_on_scheduled_for;
DROP INDEX IF EXISTS index_queue_on_tag;
DROP INDEX IF EXISTS index_queue_on_running;
DROP INDEX IF EXISTS index_queue_on_created;


CREATE INDEX IF NOT EXISTS root_queue_index_by_path ON queue (workspace_id, created_at);
CREATE INDEX IF NOT EXISTS root_queue_index ON queue (job_kind, tag, scheduled_for, created_at) WHERE running is false;
CREATE INDEX IF NOT EXISTS root_queue_index_suspended ON queue (job_kind, tag, suspend_until, suspend, scheduled_for, created_at) WHERE suspend_until is not null;

CREATE INDEX IF NOT EXISTS concurrency_limit_stats_queue ON queue (workspace_id, script_path, started_at) WHERE concurrent_limit is not null;
CREATE INDEX IF NOT EXISTS concurrency_limit_stats_completed_job ON completed_job (workspace_id, script_path, started_at);