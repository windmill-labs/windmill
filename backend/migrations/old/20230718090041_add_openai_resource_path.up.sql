-- Add up migration script here
ALTER TABLE workspace_settings ADD COLUMN openai_resource_path VARCHAR(1000);
ALTER TABLE workspace_settings DROP COLUMN openai_key;