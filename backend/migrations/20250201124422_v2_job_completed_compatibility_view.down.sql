-- Add down migration script here
DROP VIEW IF EXISTS completed_job;
ALTER TABLE IF EXISTS v2_job_completed RENAME TO completed_job;
