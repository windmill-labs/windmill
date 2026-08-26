-- Serves the suspended-job pull in windmill-common/src/worker.rs, whose resume test is the
-- indexed CASE expression. Two things about the shape are load-bearing:
--   * (priority DESC NULLS LAST, created_at) leads, so the scan yields that query's ORDER BY
--     and stops at the first match rather than sorting.
--   * the index is dropped before it is built rather than relying on IF NOT EXISTS. The
--     OVERRIDDEN_MIGRATIONS rewrite in windmill-api/src/db.rs runs these CONCURRENTLY, and an
--     interrupted concurrent build leaves the index present but invalid, which IF NOT EXISTS
--     would then skip rebuilding. Retiring the index this replaces is left to the migration
--     that follows, so this one can only ever be replayed while that index is still there to
--     cover the rebuild.
DROP INDEX IF EXISTS queue_suspended_v2;

CREATE INDEX IF NOT EXISTS queue_suspended_v2
    ON v2_job_queue (
        priority DESC NULLS LAST,
        created_at,
        (CASE WHEN suspend <= 0 THEN '-infinity'::timestamptz ELSE suspend_until END),
        tag
    )
    WHERE suspend_until IS NOT NULL;
