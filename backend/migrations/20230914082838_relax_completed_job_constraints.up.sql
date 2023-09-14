-- Add up migration script here
ALTER TABLE completed_job ALTER COLUMN duration_ms TYPE bigint;
ALTER TABLE completed_job ALTER COLUMN email TYPE VARCHAR(255);
ALTER TABLE queue ALTER COLUMN email TYPE VARCHAR(255);
ALTER TABLE queue ALTER COLUMN canceled_by TYPE VARCHAR(255);
