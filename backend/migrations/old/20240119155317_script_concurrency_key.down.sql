-- Add down migration script here
ALTER TABLE script DROP COLUMN concurrency_key;
