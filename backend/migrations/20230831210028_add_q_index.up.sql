-- Add up migration script here
CREATE INDEX IF NOT EXISTS queue_sort ON queue (scheduled_for, created_at) WHERE running = false;