-- Add down migration script here
ALTER TABLE queue DROP COLUMN raw_flow;
ALTER TABLE completed_job DROP COLUMN raw_flow;

ALTER TABLE queue DROP COLUMN is_flow_step NOT NULL DEFAULT false;
ALTER TABLE completed_job DROP COLUMN is_flow_step NOT NULL DEFAULT false;
