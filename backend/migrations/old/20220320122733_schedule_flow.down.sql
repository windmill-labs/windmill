-- Add down migration script here
ALTER TABLE schedule ADD COLUMN script_hash BIGINT;
ALTER TABLE schedule DROP COLUMN is_flow;

