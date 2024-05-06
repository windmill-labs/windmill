-- Add up migration script here
CREATE TABLE custom_concurrency_key (
    job_id UUID PRIMARY KEY REFERENCES queue(id),
    concurrency_key VARCHAR(255) NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);
