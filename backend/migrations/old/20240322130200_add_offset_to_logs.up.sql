-- Add up migration script here
ALTER TABLE job_logs ADD COLUMN log_offset int NOT NULL DEFAULT 0;
ALTER TABLE job_logs ADD COLUMN log_file_index text[];

