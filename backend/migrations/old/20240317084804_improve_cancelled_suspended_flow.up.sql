-- Add up migration script here
ALTER TABLE resume_job ADD COLUMN approved BOOLEAN NOT NULL DEFAULT true;