CREATE SEQUENCE debounce_batch_seq START 1;

CREATE TABLE IF NOT EXISTS v2_job_debounce_batch(
    id UUID PRIMARY KEY,
    debounce_batch BIGINT NOT NULL DEFAULT nextval('debounce_batch_seq')
);

CREATE INDEX IF NOT EXISTS idx_v2_job_debounce_batch_debounce_batch ON v2_job_debounce_batch(debounce_batch);

ALTER TABLE debounce_key ADD COLUMN IF NOT EXISTS first_started_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT now();
ALTER TABLE debounce_key ADD COLUMN IF NOT EXISTS debounced_times INTEGER NOT NULL DEFAULT 0;

