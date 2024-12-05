-- Add up migration script here
ALTER TABLE app ADD COLUMN custom_path TEXT CHECK (custom_path ~ '^[\w-]+(\/[\w-]+)*$');
