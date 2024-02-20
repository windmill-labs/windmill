-- Add down migration script here
ALTER TABLE queue DROP COLUMN flow_last_progress_ts;
