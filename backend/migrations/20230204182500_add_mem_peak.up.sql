-- Add up migration script here
ALTER TABLE queue ADD COLUMN mem_peak INTEGER;
ALTER TABLE completed_job ADD COLUMN mem_peak INTEGER;
