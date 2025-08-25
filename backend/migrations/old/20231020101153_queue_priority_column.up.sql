-- Add up migration script here
ALTER TABLE queue ADD COLUMN priority SMALLINT;
DROP INDEX IF EXISTS queue_sort;
CREATE INDEX IF NOT EXISTS queue_sort ON queue (priority DESC NULLS LAST, scheduled_for, created_at, tag) WHERE running = false;
DROP INDEX IF EXISTS queue_suspended;
CREATE INDEX IF NOT EXISTS queue_suspended ON queue (priority DESC NULLS LAST, created_at, suspend_until, suspend, tag) WHERE suspend_until is not null;

ALTER TABLE completed_job ADD COLUMN priority SMALLINT;
ALTER TABLE script ADD COLUMN priority SMALLINT;
