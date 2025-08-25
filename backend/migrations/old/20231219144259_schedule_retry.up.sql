-- Add up migration script here
ALTER TABLE schedule ADD COLUMN IF NOT EXISTS retry JSONB;
ALTER TYPE JOB_KIND ADD VALUE IF NOT EXISTS 'singlescriptflow';
