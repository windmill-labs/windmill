-- Add up migration script here
CREATE INDEX IF NOT EXISTS idx_debounce_key_job_id ON debounce_key (job_id);
