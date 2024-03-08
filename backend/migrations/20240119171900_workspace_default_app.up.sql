-- Add up migration script here
ALTER TABLE workspace_settings ADD COLUMN default_app VARCHAR(255);
