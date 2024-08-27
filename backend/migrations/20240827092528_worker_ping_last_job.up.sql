-- Add up migration script here
ALTER TABLE worker_ping RENAME COLUMN current_job_id TO last_job_id;
ALTER TABLE worker_ping RENAME COLUMN current_job_workspace_id TO last_job_workspace_id;