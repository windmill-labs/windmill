-- Add up migration script here
CREATE INDEX IF NOT EXISTS index_queue_on_tag ON queue (tag);
CREATE INDEX IF NOT EXISTS index_completed_job_on_schedule_path ON completed_job (schedule_path);