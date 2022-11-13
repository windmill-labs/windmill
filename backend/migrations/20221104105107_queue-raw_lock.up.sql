-- Add up migration script here
ALTER TABLE queue ADD COLUMN raw_lock TEXT DEFAULT NULL;