-- Claim-based, exactly-once consumption of debounce batches.
-- A batch row is "claimed" by the survivor that accumulates its args. Stamping the
-- row consumed (instead of deleting it) lets a later-pulled survivor of the same
-- batch tell "my contribution was already processed" (consumed_at set -> no-op)
-- apart from "I was never batched" (no row at all; CE / legacy -> run my own args).
-- consumed_at doubles as the GC timestamp. NULL = not yet consumed.
-- consumed_by records which job claimed the row, so a job that is re-pulled (e.g.
-- crash recovery) can tell its own prior claim (keep its accumulated args) apart from
-- a sibling survivor having swept it in (run empty).
ALTER TABLE v2_job_debounce_batch
    ADD COLUMN IF NOT EXISTS consumed_at TIMESTAMP WITH TIME ZONE,
    ADD COLUMN IF NOT EXISTS consumed_by UUID;

-- Keeps the GC sweep (delete consumed rows past a grace period) cheap.
CREATE INDEX IF NOT EXISTS idx_v2_job_debounce_batch_consumed_at
    ON v2_job_debounce_batch (consumed_at)
    WHERE consumed_at IS NOT NULL;
