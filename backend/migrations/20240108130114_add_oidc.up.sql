-- Add up migration script here
ALTER TABLE token ADD COLUMN IF NOT EXISTS job UUID;