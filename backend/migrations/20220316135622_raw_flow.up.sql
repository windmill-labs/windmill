-- Add up migration script here
ALTER TABLE queue ADD COLUMN raw_flow JSONB;
ALTER TABLE completed_job ADD COLUMN raw_flow JSONB;

ALTER TABLE queue ADD COLUMN is_flow_step boolean DEFAULT false;
ALTER TABLE completed_job ADD COLUMN is_flow_step boolean DEFAULT false;
