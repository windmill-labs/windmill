-- Add up migration script here
ALTER TABLE workspace_settings ADD COLUMN code_completion_enabled BOOLEAN NOT NULL DEFAULT false;