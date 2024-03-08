-- Add up migration script here
ALTER TABLE resume_job
DROP CONSTRAINT resume_job_value_check;
