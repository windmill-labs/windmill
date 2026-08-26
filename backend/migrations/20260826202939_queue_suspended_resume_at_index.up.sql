-- Serves the suspended-job pull in windmill-common/src/worker.rs, whose resume test is the
-- indexed CASE expression. Three things about the shape are load-bearing:
--   * (priority DESC NULLS LAST, created_at) leads, so the scan yields that query's ORDER BY
--     and stops at the first match rather than sorting.
--   * suspend_until is a column and not only the partial predicate, otherwise the query's
--     IS NOT NULL guard becomes a heap filter costing one fetch per suspended row.
--   * the index is dropped before it is built rather than relying on IF NOT EXISTS. The
--     OVERRIDDEN_MIGRATIONS rewrite in windmill-api/src/db.rs runs these CONCURRENTLY, an
--     interrupted concurrent build leaves the index present but invalid, IF NOT EXISTS would
--     skip rebuilding it on the retry, and the DROP below would then retire the only usable
--     index the pull has.
DROP INDEX IF EXISTS queue_suspended_v2;

CREATE INDEX IF NOT EXISTS queue_suspended_v2
    ON v2_job_queue (
        priority DESC NULLS LAST,
        created_at,
        (CASE WHEN suspend <= 0 THEN '-infinity'::timestamptz ELSE suspend_until END),
        suspend_until,
        tag
    )
    WHERE suspend_until IS NOT NULL;

DROP INDEX IF EXISTS queue_suspended;
