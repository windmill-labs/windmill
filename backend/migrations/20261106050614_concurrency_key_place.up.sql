-- A job's place in line on its concurrency key: its scheduled_for when queued, or, once a
-- limit defers it, the scheduled_for it had before. Lives on the key row so that the line can
-- be read from an index rather than by sorting the key's backlog against v2_job_queue.
ALTER TABLE concurrency_key ADD COLUMN IF NOT EXISTS place TIMESTAMPTZ;

UPDATE concurrency_key ck
SET place = COALESCE(ck.original_scheduled_for, q.scheduled_for)
FROM v2_job_queue q
WHERE q.id = ck.job_id AND ck.ended_at IS NULL AND ck.place IS NULL;
