-- Add down migration script here
ALTER TABLE workspace_settings
ALTER COLUMN ai_resource_path TYPE text
USING ai_resource_path->>'path';