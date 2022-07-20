-- Add up migration script here
ALTER TABLE script ADD COLUMN trigger_reco_interval INTEGER;
ALTER TABLE completed_job ADD COLUMN is_skipped BOOLEAN NOT NULL DEFAULT FALSE;
