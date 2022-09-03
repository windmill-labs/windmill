-- Add up migration script here
ALTER TABLE audit ALTER COLUMN username TYPE varchar(255);
