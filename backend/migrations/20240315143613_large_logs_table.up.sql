-- Add up migration script here
CREATE TABLE IF NOT EXISTS job_logs
(
    job_id uuid PRIMARY KEY,
    workspace_id VARCHAR(255),
    created_at TIMESTAMP WITH TIME ZONE DEFAULT now(),
    logs TEXT
);