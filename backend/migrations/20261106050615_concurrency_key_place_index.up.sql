-- Serves the in-order admission check and wake in windmill-queue/src/jobs.rs, which read the
-- line of a concurrency key and tag from the front and stop after the admitted jobs and a few
-- more. Without it, every admission of an in-order key sorts the key's whole backlog.
-- Dropped before it is built because the OVERRIDDEN_MIGRATIONS rewrite in
-- windmill-api/src/db.rs runs both CONCURRENTLY: an interrupted concurrent build leaves an
-- invalid index that IF NOT EXISTS would skip rebuilding.
DROP INDEX IF EXISTS concurrency_key_place_idx;

CREATE INDEX IF NOT EXISTS concurrency_key_place_idx
    ON concurrency_key (key, tag, priority DESC, place, job_id)
    WHERE ended_at IS NULL AND place IS NOT NULL;
