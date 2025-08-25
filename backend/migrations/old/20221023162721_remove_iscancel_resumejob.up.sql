-- Add up migration script here
ALTER TABLE resume_job DROP COLUMN is_cancel;
ALTER TABLE resume_job ADD COLUMN approver VARCHAR(50);