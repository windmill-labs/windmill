-- Add up migration script here
ALTER TABLE script ADD COLUMN concurrency_key VARCHAR(255);
