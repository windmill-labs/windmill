-- Add up migration script here
DROP INDEX IF EXISTS queue_sort;
CREATE INDEX IF NOT EXISTS queue_sort ON queue (scheduled_for, created_at, tag) WHERE running = false;

CREATE TABLE IF NOT EXISTS concurrency_counter (
  concurrency_id VARCHAR(1000) PRIMARY KEY,
  counter INTEGER NOT NULL
);

DROP INDEX IF EXISTS root_queue_index_suspended;

DROP INDEX IF EXISTS queue_suspended;
CREATE INDEX IF NOT EXISTS queue_suspended ON queue (created_at, suspend_until, suspend, tag) WHERE suspend_until is not null;
