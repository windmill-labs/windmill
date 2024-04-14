-- Add up migration script here
ALTER TABLE flow ADD COLUMN concurrency_key VARCHAR(255);
