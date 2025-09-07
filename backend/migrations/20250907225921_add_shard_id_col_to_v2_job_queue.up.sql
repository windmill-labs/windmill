-- Add up migration script here
ALTER TABLE v2_job_queue
    ADD COLUMN shard_id BIGINT DEFAULT NULL;

DROP INDEX IF EXISTS queue_suspended;

CREATE INDEX queue_suspended 
    ON public.v2_job_queue (shard_id, priority DESC NULLS LAST, created_at, suspend_until, suspend, tag) 
    WHERE (suspend_until IS NOT NULL);

DROP INDEX IF EXISTS queue_sort_v2;

CREATE INDEX queue_sort_v2 
    ON public.v2_job_queue (shard_id, priority DESC NULLS LAST, scheduled_for, tag) 
    WHERE (running = false AND shard_id is NULL);