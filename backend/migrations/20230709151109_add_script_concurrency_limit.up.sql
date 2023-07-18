-- Add up migration script here
ALTER TABLE script ADD COLUMN concurrent_limit INTEGER;
ALTER TABLE script ADD COLUMN concurrency_time_window_s INTEGER;
ALTER TABLE queue ADD COLUMN concurrent_limit INTEGER;
ALTER TABLE queue ADD COLUMN concurrency_time_window_s INTEGER;
