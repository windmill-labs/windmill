-- Serves the worker pull queries in windmill-common/src/worker.rs, which order by
-- COALESCE(priority, -1) DESC: a job with no priority sorts at -1, so a negative (low)
-- priority runs after it while 0 (flow steps) and above run before it. The expression must
-- match the queries' ORDER BY exactly or every poll falls back to a sort of the whole queue.
-- Dropped before it is built rather than relying on IF NOT EXISTS: the OVERRIDDEN_MIGRATIONS
-- rewrite in windmill-api/src/db.rs runs this CONCURRENTLY, and an interrupted concurrent
-- build leaves an invalid index that IF NOT EXISTS would skip. queue_sort_v2 is kept for the
-- previous release's workers, which still pull with `priority DESC NULLS LAST`.
DROP INDEX IF EXISTS queue_sort_v3;

CREATE INDEX IF NOT EXISTS queue_sort_v3
    ON v2_job_queue ((COALESCE(priority, -1)) DESC, scheduled_for, tag)
    WHERE running = false;
