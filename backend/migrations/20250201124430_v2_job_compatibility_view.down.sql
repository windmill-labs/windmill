-- Add down migration script here
DROP VIEW IF EXISTS job;
ALTER TABLE IF EXISTS v2_job RENAME TO job;
