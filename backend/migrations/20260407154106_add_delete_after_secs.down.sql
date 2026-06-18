DROP TABLE IF EXISTS job_delete_schedule;
ALTER TABLE script DROP COLUMN IF EXISTS delete_after_secs;
