-- Add up migration script here
ALTER TABLE workspace_settings
ALTER COLUMN openai_resource_path TYPE jsonb
USING jsonb_build_object('provider', 'openai', 'path', openai_resource_path);

ALTER TABLE workspace_settings
RENAME COLUMN openai_resource_path TO ai_resource;
