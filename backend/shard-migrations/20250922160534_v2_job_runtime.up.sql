-- Add up migration script here
CREATE TABLE IF NOT EXISTS v2_job_runtime (
    id UUID PRIMARY KEY REFERENCES v2_job(id) ON DELETE CASCADE,
    ping TIMESTAMP WITH TIME ZONE,
    memory_peak INTEGER
);