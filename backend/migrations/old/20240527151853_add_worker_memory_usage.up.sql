-- Add up migration script here
ALTER TABLE worker_ping ADD COLUMN memory_usage BIGINT;