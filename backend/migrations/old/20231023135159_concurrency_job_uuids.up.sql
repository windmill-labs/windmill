-- Add up migration script here
ALTER TABLE concurrency_counter ADD COLUMN job_uuids jsonb NOT NULL DEFAULT '{}'::jsonb;
ALTER TABLE concurrency_counter DROP COLUMN counter;
