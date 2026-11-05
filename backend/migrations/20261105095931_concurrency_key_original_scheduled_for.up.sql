-- The job's scheduled_for before a concurrency limit first deferred it: its place in line
-- when concurrency-limited jobs are admitted in order.
ALTER TABLE concurrency_key ADD COLUMN IF NOT EXISTS original_scheduled_for TIMESTAMPTZ;
