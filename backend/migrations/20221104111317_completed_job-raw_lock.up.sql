-- Add up migration script here
ALTER TABLE completed_job ADD COLUMN raw_lock TEXT DEFAULT NULL;