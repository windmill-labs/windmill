-- Add down migration script here
ALTER TABLE concurrency_counter ADD COLUMN counter INTEGER NOT NULL DEFAULT 0;
ALTER TABLE concurrency_counter DROP COLUMN job_uuids;
