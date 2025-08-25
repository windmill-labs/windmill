-- Add down migration script here
ALTER TABLE script DROP COLUMN concurrent_limit;
ALTER TABLE script DROP COLUMN concurrency_time_window_s;
