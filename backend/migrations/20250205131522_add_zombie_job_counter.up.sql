-- Add up migration script here

CREATE TABLE IF NOT EXISTS zombie_job_counter (
    job_id UUID PRIMARY KEY REFERENCES v2_job (id) ON DELETE CASCADE,
    counter INTEGER NOT NULL DEFAULT 0
);

