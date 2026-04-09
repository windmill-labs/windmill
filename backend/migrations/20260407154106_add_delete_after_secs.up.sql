ALTER TABLE script ADD COLUMN delete_after_secs INTEGER;

CREATE TABLE job_delete_schedule (
    job_id UUID PRIMARY KEY,
    workspace_id VARCHAR(50) NOT NULL,
    delete_at TIMESTAMPTZ NOT NULL
);

CREATE INDEX idx_job_delete_schedule_delete_at ON job_delete_schedule (delete_at);
