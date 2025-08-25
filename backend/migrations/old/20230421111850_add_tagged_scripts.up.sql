-- Add up migration script here
ALTER TABLE script ADD COLUMN tag VARCHAR(50);
ALTER TABLE completed_job  ADD COLUMN tag VARCHAR(50) NOT NULL DEFAULT 'other';
ALTER TABLE queue ADD COLUMN tag VARCHAR(50) NOT NULL DEFAULT 'other';