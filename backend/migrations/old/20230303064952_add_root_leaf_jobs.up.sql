-- Add up migration script here
ALTER TABLE queue ADD COLUMN root_job uuid;
ALTER TABLE queue ADD COLUMN leaf_jobs jsonb;
