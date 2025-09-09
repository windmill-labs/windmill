-- Add down migration script here
ALTER TABLE v2_job_queue
    DROP COLUMN shard_id;

DROP INDEX IF EXISTS queue_suspended;

CREATE INDEX queue_suspended 
    ON v2_job_queue (priority DESC NULLS LAST, created_at, suspend_until, suspend, tag) 
    WHERE (suspend_until IS NOT NULL);

DROP INDEX IF EXISTS queue_sort_v2;

CREATE INDEX queue_sort_v2 
    ON v2_job_queue (priority DESC NULLS LAST, scheduled_for, tag) 
    WHERE (running = false);