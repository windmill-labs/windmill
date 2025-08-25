-- Add up migration script here
ALTER TABLE draft
ALTER COLUMN value TYPE json;