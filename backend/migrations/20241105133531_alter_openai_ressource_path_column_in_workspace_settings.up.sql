-- Add up migration script here
ALTER TABLE workspace_settings
ALTER COLUMN ai_resource_path TYPE jsonb
USING jsonb_build_object('resource_type', 'openai', 'path', ai_resource_path);

ALTER TABLE workspace_settings
RENAME COLUMN ai_resource_path TO ai_resource;
