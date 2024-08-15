-- Add up migration script here
ALTER TABLE workspace_settings ADD COLUMN IF NOT EXISTS deploy_ui JSONB;
