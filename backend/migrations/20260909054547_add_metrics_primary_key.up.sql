-- The seventh table. Why any of them need a key is in
-- 20260909052532_add_missing_primary_keys, which covers the other six.
--
-- It is separate from them because it was two orders of magnitude larger than all
-- six put together (~400 MB / 750k rows on the instance this was measured on) and
-- the ALTER rewrites it under ACCESS EXCLUSIVE. A migration is one transaction and
-- Postgres holds its locks until commit, so alone it locks none of the six while it
-- rewrites, and an instance that struggles with this one keeps their keys on retry.
--
-- The surrogate cannot be called `id`: `metrics.id` already exists and holds the
-- metric NAME, repeated once per sample.

ALTER TABLE metrics
    ADD COLUMN IF NOT EXISTS row_id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY;
