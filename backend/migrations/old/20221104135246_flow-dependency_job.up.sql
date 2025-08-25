-- Add up migration script here
ALTER TABLE flow ADD COLUMN dependency_job UUID DEFAULT NULL;
ALTER TYPE JOB_KIND ADD VALUE IF NOT EXISTS 'flowdependencies';