-- Add up migration script here
CREATE TABLE IF NOT EXISTS job_logs (
    job_id UUID NOT NULL,
    workspace_id VARCHAR(100) NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    logs TEXT,
    log_offset INTEGER,
    log_file_index TEXT[],
    PRIMARY KEY (job_id, log_offset)
);

-- Create indices for job_logs
CREATE INDEX IF NOT EXISTS job_logs_job_id_idx ON job_logs (job_id);
CREATE INDEX IF NOT EXISTS job_logs_workspace_id_created_at_idx ON job_logs (workspace_id, created_at DESC);