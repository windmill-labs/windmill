-- Add up migration script here
ALTER TABLE v2_job_queue ALTER COLUMN __created_by SET NOT NULL;
