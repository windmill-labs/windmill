-- Add up migration script here
ALTER TABLE workspace_settings
ADD COLUMN auto_add boolean default false;