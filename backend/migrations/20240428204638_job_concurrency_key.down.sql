-- Add down migration script here
DROP INDEX concurrency_key_ended_at_idx;

ALTER TABLE concurrency_key
    DROP CONSTRAINT concurrency_key_pkey;

ALTER TABLE concurrency_key
    RENAME TO custom_concurrency_key_ended;

ALTER TABLE custom_concurrency_key_ended
    DROP COLUMN job_id,
    ADD PRIMARY KEY (key, ended_at);


