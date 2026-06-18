-- Add up migration script here
ALTER TABLE app_version ADD COLUMN IF NOT EXISTS raw_app BOOLEAN NOT NULL DEFAULT FALSE;