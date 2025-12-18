ALTER TABLE debounce_key DROP COLUMN IF EXISTS debounced_times;
ALTER TABLE debounce_key DROP COLUMN IF EXISTS first_started_at;
DROP INDEX IF EXISTS idx_v2_job_debounce_batch_debounce_batch;
DROP TABLE IF EXISTS v2_job_debounce_batch;
DROP SEQUENCE IF EXISTS debounce_batch_seq;
