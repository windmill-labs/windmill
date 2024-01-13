-- Add up migration script here
UPDATE workspace_settings SET git_sync = '[]'::jsonb || git_sync 
