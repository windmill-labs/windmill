-- Add up migration script here
ALTER TABLE queue ADD COLUMN flow_last_progress_ts TIMESTAMP WITH TIME ZONE;
