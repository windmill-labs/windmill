-- Add up migration script here
CREATE TABLE IF NOT EXISTS outstanding_wait_time (
    job_id UUID PRIMARY KEY,
    self_wait_time_ms BIGINT DEFAULT NULL,
    aggregate_wait_time_ms BIGINT DEFAULT NULL
);