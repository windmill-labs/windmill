-- Add down migration script here
ALTER TABLE job_logs DROP COLUMN log_offset
ALTER TABLE job_logs DROP COLUMN log_file_index;
