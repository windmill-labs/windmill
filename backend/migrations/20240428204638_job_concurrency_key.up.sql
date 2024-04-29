-- Add up migration script here
ALTER TABLE custom_concurrency_key_ended
    RENAME TO concurrency_key;

DROP EXTENSION "uuid-ossp";
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

ALTER TABLE concurrency_key
    DROP CONSTRAINT custom_concurrency_key_ended_pkey,
    -- uuid_generate_v4 was not present until i dropped and reloaded uuid-ossp extension
    ADD COLUMN job_id UUID DEFAULT uuid_generate_v4() NOT NULL PRIMARY KEY,
    -- ADD COLUMN job_id UUID DEFAULT (SELECT md5(random()::text || clock_timestamp()::text)::uuid) NOT NULL PRIMARY KEY,
    ADD COLUMN concurrency_time_window_s INTEGER DEFAULT 0 NOT NULL,
    ALTER COLUMN ended_at DROP NOT NULL,
    ALTER COLUMN ended_at SET DEFAULT NULL;


CREATE INDEX concurrency_key_ended_at_idx ON concurrency_key (key, ended_at);
