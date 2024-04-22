-- Add up migration script here
CREATE TABLE custom_concurrency_key (
    job_id UUID PRIMARY KEY,
    concurrency_key VARCHAR(255) NOT NULL
);
