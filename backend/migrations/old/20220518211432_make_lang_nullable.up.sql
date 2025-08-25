-- Add up migration script here
ALTER TABLE queue
ALTER COLUMN language DROP NOT NULL;

ALTER TABLE completed_job
ALTER COLUMN language DROP NOT NULL;
