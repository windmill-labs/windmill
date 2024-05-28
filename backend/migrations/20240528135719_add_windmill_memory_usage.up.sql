-- Add up migration script here
ALTER TABLE worker_ping ADD COLUMN wm_memory_usage BIGINT;