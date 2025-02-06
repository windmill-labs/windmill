-- Add down migration script here
DROP VIEW queue;
ALTER TABLE v2_job_queue RENAME TO queue;
