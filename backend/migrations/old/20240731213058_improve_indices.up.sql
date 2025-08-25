-- Add up migration script here
DROP INDEX IF EXISTS queue_sort;
CREATE INDEX IF NOT EXISTS queue_sort ON queue (running, tag, priority DESC NULLS LAST, scheduled_for, created_at) WHERE running = false;
CREATE INDEX IF NOT EXISTS queue_sort_2 ON queue (running, priority DESC NULLS LAST, scheduled_for, created_at) WHERE running = false;