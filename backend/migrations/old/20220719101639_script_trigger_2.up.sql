-- Add up migration script here
ALTER TABLE script DROP COLUMN trigger_reco_interval;
ALTER TABLE script ADD COLUMN is_trigger BOOLEAN NOT NULL DEFAULT false;

