-- Add up migration script here
ALTER TABLE queue
ALTER COLUMN last_ping DROP NOT NULL;