-- Add up migration script here
CREATE TABLE outstanding_wait_time (
    job_id UUID PRIMARY KEY,
    waiting_time_ms BIGINT NOT NULL
);
