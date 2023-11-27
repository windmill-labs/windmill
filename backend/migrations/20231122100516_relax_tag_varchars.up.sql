-- Add up migration script here
ALTER TABLE queue ALTER COLUMN tag TYPE varchar(255);
ALTER TABLE completed_job ALTER COLUMN tag TYPE varchar(255);