-- The seventh table; why any of them need a key is in
-- 20260909052532_add_missing_primary_keys.
--
-- Kept out of that migration because it was two orders of magnitude larger than all
-- six together and its ALTER rewrites it under ACCESS EXCLUSIVE: one migration is
-- one transaction, so alone it holds no lock on them while it rewrites.
--
-- The surrogate cannot be called `id` -- `metrics.id` holds the metric NAME.

ALTER TABLE metrics
    ADD COLUMN IF NOT EXISTS row_id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY;
