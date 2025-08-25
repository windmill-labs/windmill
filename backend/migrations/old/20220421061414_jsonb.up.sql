-- Add up migration script here
ALTER TABLE script
ALTER COLUMN schema TYPE json;

ALTER TABLE flow
ALTER COLUMN schema TYPE json;
