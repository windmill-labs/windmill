-- Add up migration script here
ALTER TABLE queue ADD COLUMN same_worker BOOLEAN DEFAULT FALSE;
