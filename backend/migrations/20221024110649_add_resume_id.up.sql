-- Add up migration script here
ALTER TABLE resume_job ADD COLUMN resume_id INTEGER NOT NULL DEFAULT 0;
ALTER TYPE JOB_KIND ADD VALUE 'identity';