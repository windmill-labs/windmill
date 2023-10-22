-- Add up migration script here
ALTER TABLE queue DROP COLUMN priority;
DROP INDEX IF EXISTS queue_sort;
CREATE INDEX IF NOT EXISTS queue_sort ON queue (scheduled_for, created_at, tag) WHERE running = false;
DROP INDEX IF EXISTS queue_suspended;
CREATE INDEX IF NOT EXISTS queue_suspended ON queue (created_at, suspend_until, suspend, tag) WHERE suspend_until is not null;

ALTER TABLE completed_job DROP COLUMN priority;
ALTER TABLE script DROP COLUMN priority;
