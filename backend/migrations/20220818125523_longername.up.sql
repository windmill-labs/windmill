-- Add up migration script here
ALTER TABLE queue ALTER COLUMN created_by TYPE varchar(255);
ALTER TABLE completed_job ALTER COLUMN created_by TYPE varchar(255);
