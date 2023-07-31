-- Add up migration script here
ALTER TABLE queue ADD COLUMN timeout INTEGER;
