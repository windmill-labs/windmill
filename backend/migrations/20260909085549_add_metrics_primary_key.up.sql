-- The last of the seven; why any of them need a key is in
-- 20260909052532_add_missing_primary_keys.
--
-- Kept out of that migration because it is the largest (~400 MB / 750k rows on the
-- instance this was measured on, a steady state: `queue_%` rows, which are nearly all
-- of them, are swept at 14 days) and its ALTER rewrites it under ACCESS EXCLUSIVE. One
-- migration is one transaction, so alone it holds no lock on the others as it rewrites.
--
-- The surrogate cannot be called `id` -- `metrics.id` holds the metric NAME.

ALTER TABLE metrics
    ADD COLUMN IF NOT EXISTS row_id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY;
