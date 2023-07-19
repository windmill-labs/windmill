-- Add down migration script here
ALTER TABLE workspace_settings DROP COLUMN openai_resource_path;
ALTER TABLE workspace_settings ADD COLUMN openai_key VARCHAR(255);