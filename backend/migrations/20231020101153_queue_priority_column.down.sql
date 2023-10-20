-- Add up migration script here
ALTER TABLE queue DROP COLUMN priority;
DROP INDEX IF EXISTS queue_sort;
CREATE INDEX IF NOT EXISTS queue_sort ON queue (scheduled_for, created_at, tag) WHERE running = false;

ALTER TABLE script DROP COLUMN priority;
