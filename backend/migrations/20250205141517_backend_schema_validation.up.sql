-- Add up migration script here
ALTER TABLE script
ADD COLUMN schema_validation BOOLEAN NOT NULL DEFAULT FALSE;
