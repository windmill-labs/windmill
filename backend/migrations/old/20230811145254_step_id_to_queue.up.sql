-- Add up migration script here
ALTER TABLE queue ADD COLUMN flow_step_id VARCHAR(255);