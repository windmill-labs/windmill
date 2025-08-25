-- Add down migration script here
UPDATE workspace_settings SET git_sync = git_sync->0;