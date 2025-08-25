-- Add up migration script here
ALTER TABLE resume_job ALTER COLUMN approver TYPE VARCHAR(1000);