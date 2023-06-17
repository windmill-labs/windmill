-- Add up migration script here
ALTER TABLE workspace_settings ADD COLUMN deploy_to VARCHAR(255);
