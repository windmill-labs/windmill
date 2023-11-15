-- Add up migration script here
ALTER TABLE workspace_settings ADD COLUMN IF NOT EXISTS large_file_storage JSONB;