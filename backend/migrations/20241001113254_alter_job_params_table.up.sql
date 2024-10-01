-- Add up migration script here
ALTER TABLE job_params ADD COLUMN workspace_id VARCHAR(50);

-- args has been moved to job_params table
ALTER TABLE job_params DROP COLUMN args;