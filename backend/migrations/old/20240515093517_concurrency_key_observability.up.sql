-- Add up migration script here
CREATE TABLE concurrency_key (
    key VARCHAR(255) NOT NULL,
    ended_at TIMESTAMP WITH TIME ZONE,
    job_id uuid NOT NULL,
    PRIMARY KEY (job_id)
);


CREATE INDEX concurrency_key_ended_at_idx ON concurrency_key (key, ended_at DESC);
