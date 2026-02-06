-- v2_job_queue: drop obsolete indexes and create new ones CONCURRENTLY
DROP INDEX CONCURRENTLY IF EXISTS concurrency_limit_stats_queue;
DROP INDEX CONCURRENTLY IF EXISTS queue_sort;
DROP INDEX CONCURRENTLY IF EXISTS queue_sort_2;

CREATE INDEX CONCURRENTLY IF NOT EXISTS queue_sort_v2
    ON v2_job_queue (priority DESC NULLS LAST, scheduled_for, tag)
    WHERE running = false;

CREATE INDEX CONCURRENTLY IF NOT EXISTS v2_job_queue_suspend
    ON v2_job_queue (workspace_id, suspend)
    WHERE suspend > 0;
