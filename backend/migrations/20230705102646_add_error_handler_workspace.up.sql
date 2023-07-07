-- Add up migration script here
ALTER TABLE workspace_settings ADD COLUMN error_handler VARCHAR(1000);