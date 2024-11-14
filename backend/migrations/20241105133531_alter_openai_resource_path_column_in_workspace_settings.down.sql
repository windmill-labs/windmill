-- Add down migration script here
ALTER TABLE workspace_settings
ALTER COLUMN ai_resource TYPE text
USING ai_resource->>'path';

ALTER TABLE workspace_settings
RENAME COLUMN ai_resource to openai_resource_path;