-- Add down migration script here
ALTER TABLE flow DROP COLUMN concurrency_key;
