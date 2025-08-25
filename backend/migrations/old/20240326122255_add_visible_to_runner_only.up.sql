-- Add up migration script here
ALTER TABLE script ADD COLUMN visible_to_runner_only BOOLEAN;
ALTER TABLE flow ADD COLUMN visible_to_runner_only BOOLEAN;