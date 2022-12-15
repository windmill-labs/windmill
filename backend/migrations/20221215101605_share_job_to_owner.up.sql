-- Add up migration script here
ALTER TABLE queue ADD COLUMN visible_to_owner BOOLEAN DEFAULT true;
ALTER TABLE completed_job ADD COLUMN visible_to_owner BOOLEAN DEFAULT true;
