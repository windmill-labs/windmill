-- Add up migration script here
ALTER TABLE schedule ADD COLUMN on_failure VARCHAR(1000) DEFAULT NULL;
