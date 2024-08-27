-- Add down migration script here
ALTER TABLE worker_ping RENAME COLUMN last_job_id TO current_job_id;
ALTER TABLE worker_ping RENAME COLUMN last_job_workspace_id TO current_job_workspace_id;