-- Add up migration script here
ALTER TABLE custom_concurrency_key_ended
    RENAME TO concurrency_key;

ALTER TABLE concurrency_key
    DROP CONSTRAINT custom_concurrency_key_ended_pkey,
    ADD COLUMN job_id UUID DEFAULT gen_random_uuid() NOT NULL PRIMARY KEY,
    ALTER COLUMN ended_at DROP NOT NULL,
    ALTER COLUMN ended_at SET DEFAULT NULL;

CREATE INDEX concurrency_key_ended_at_idx ON concurrency_key (key, ended_at DESC);
