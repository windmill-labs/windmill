-- Add down migration script here
ALTER TABLE job_params DROP COLUMN workspace_id;

-- args has been moved to job_params table
ALTER TABLE job_params ADD COLUMN args JSONB;