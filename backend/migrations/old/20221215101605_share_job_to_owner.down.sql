-- Add down migration script here
ALTER TABLE queue DROP COLUMN visible_to_owner;
ALTER TABLE completed_job DROP COLUMN visible_to_owner;

DROP POLICY see_own_path ON queue;