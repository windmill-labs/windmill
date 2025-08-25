-- Add up migration script here
ALTER TABLE password ALTER COLUMN company TYPE VARCHAR(255);
ALTER TABLE password ALTER COLUMN name TYPE VARCHAR(255);
