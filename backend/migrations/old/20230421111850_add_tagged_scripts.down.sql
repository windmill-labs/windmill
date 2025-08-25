-- Add down migration script here
ALTER TABLE script DROP COLUMN tag;
ALTER TABLE completed_job DROP COLUMN tag;
ALTER TABLE queue DROP COLUMN tag;