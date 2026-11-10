-- A job's place in line on its concurrency key, for in-order admission: the tag and priority
-- it is queued under and the scheduled_for it was due at, recorded when it is first pulled with
-- in-order admission on, or when a limit first deferred it. The line is per key and tag, by
-- priority then place, the order workers pull jobs in; it lives on the key row so that it can
-- be read from an index rather than by sorting the key's backlog. A NULL priority is recorded as
-- the lowest, as workers pull it last.
ALTER TABLE concurrency_key ADD COLUMN IF NOT EXISTS place TIMESTAMPTZ;
ALTER TABLE concurrency_key ADD COLUMN IF NOT EXISTS tag VARCHAR(255);
ALTER TABLE concurrency_key ADD COLUMN IF NOT EXISTS priority SMALLINT;

UPDATE concurrency_key ck
SET place = ck.original_scheduled_for, tag = q.tag, priority = COALESCE(q.priority, -32768)
FROM v2_job_queue q
WHERE q.id = ck.job_id AND ck.ended_at IS NULL AND ck.place IS NULL
    AND ck.original_scheduled_for IS NOT NULL;
