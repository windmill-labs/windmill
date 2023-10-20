-- Add up migration script here
ALTER TABLE queue ADD COLUMN priority SMALLINT;
DROP INDEX IF EXISTS queue_sort;
CREATE INDEX IF NOT EXISTS queue_sort ON queue (priority, scheduled_for, created_at, tag) WHERE running = false;

ALTER TABLE completed_job ADD COLUMN priority SMALLINT;
ALTER TABLE script ADD COLUMN priority SMALLINT;
