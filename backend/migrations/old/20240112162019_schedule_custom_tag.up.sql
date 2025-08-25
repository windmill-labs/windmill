-- Add up migration script here
ALTER TABLE schedule ADD COLUMN tag VARCHAR(50);
