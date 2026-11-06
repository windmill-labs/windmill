-- A job's place in line on its concurrency key: the scheduled_for it was due at when first
-- pulled with in-order admission on, or when a limit first deferred it. Lives on the key row so
-- that the line can be read from an index rather than by sorting the key's backlog.
ALTER TABLE concurrency_key ADD COLUMN IF NOT EXISTS place TIMESTAMPTZ;

UPDATE concurrency_key
SET place = original_scheduled_for
WHERE ended_at IS NULL AND place IS NULL AND original_scheduled_for IS NOT NULL;
