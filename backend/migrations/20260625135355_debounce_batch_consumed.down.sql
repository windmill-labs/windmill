DROP INDEX IF EXISTS idx_v2_job_debounce_batch_consumed_at;
ALTER TABLE v2_job_debounce_batch
    DROP COLUMN IF EXISTS consumed_at,
    DROP COLUMN IF EXISTS consumed_by;
