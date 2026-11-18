-- Serves wake_deferred_concurrency_job in windmill-queue/src/jobs.rs, which walks the jobs a
-- concurrency limit deferred on one key in the order they were first due and stops after a
-- few. Without it, every completion of a concurrency-limited job sorts the key's whole backlog.
-- Dropped before it is built because the OVERRIDDEN_MIGRATIONS rewrite in
-- windmill-api/src/db.rs runs both CONCURRENTLY: an interrupted concurrent build leaves an
-- invalid index that IF NOT EXISTS would skip rebuilding.
DROP INDEX IF EXISTS concurrency_key_waiting_idx;

CREATE INDEX IF NOT EXISTS concurrency_key_waiting_idx
    ON concurrency_key (key, original_scheduled_for, job_id)
    WHERE ended_at IS NULL AND original_scheduled_for IS NOT NULL;
