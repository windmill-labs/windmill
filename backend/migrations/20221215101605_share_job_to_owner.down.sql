-- Add down migration script here
ALTER TABLE queue DROP COLUMN visible_to_owner;
ALTER TABLE completed_job DROP COLUMN visible_to_owner;