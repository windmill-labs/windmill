-- Add up migration script here
CREATE TABLE IF NOT EXISTS v2_job_status (
    id UUID PRIMARY KEY REFERENCES v2_job(id) ON DELETE CASCADE,
    flow_status JSONB,
    flow_leaf_jobs JSONB,
    workflow_as_code_status JSONB
);