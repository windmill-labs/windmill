-- Add up migration script here
ALTER TABLE resource ALTER COLUMN created_by TYPE VARCHAR(500);