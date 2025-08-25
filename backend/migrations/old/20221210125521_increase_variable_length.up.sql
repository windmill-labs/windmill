-- Add up migration script here
ALTER TABLE variable ALTER COLUMN value TYPE VARCHAR(15000);
