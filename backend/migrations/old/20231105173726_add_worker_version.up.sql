-- Add up migration script here
ALTER TABLE worker_ping ADD COLUMN IF NOT EXISTS wm_version VARCHAR(255) NOT NULL DEFAULT '';