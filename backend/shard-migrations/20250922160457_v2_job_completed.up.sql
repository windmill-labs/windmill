-- Add up migration script here
CREATE TABLE IF NOT EXISTS v2_job_completed (
    id UUID PRIMARY KEY REFERENCES v2_job(id) ON DELETE CASCADE,
    workspace_id VARCHAR(100) NOT NULL,
    duration_ms BIGINT,
    result JSONB,
    deleted BOOLEAN NOT NULL DEFAULT FALSE,
    canceled_by VARCHAR(100),
    canceled_reason TEXT,
    flow_status JSONB,
    started_at TIMESTAMP WITH TIME ZONE,
    memory_peak INTEGER,
    status job_status NOT NULL,
    completed_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    worker VARCHAR(100),
    workflow_as_code_status JSONB,
    result_columns TEXT[],
    retries UUID[],
    extras JSONB
);

-- Create indices for v2_job_completed
CREATE INDEX IF NOT EXISTS ix_completed_job_workspace_id_started_at_new_2 ON v2_job_completed (workspace_id, started_at DESC);
CREATE INDEX IF NOT EXISTS ix_job_completed_completed_at ON v2_job_completed (completed_at DESC);
CREATE INDEX IF NOT EXISTS labeled_jobs_on_jobs ON v2_job_completed USING gin (((result -> 'wm_labels'::text))) 
    WHERE (result ? 'wm_labels'::text);