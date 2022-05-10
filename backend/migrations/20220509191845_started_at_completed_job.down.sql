-- Add down migration script here
ALTER TABLE completed_job
DROP COLUMN started_at;
