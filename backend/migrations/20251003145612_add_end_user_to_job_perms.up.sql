-- Add up migration script here
ALTER TABLE job_perms ADD COLUMN end_user_email VARCHAR(255);