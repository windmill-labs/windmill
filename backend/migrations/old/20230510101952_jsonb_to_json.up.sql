-- Add up migration script here
ALTER TABLE app_version
ALTER COLUMN value TYPE json;