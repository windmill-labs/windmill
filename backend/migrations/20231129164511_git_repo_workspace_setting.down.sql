-- Add down migration script here
ALTER TABLE workspace_settings DROP COLUMN git_sync;
ALTER TABLE script DROP COLUMN deployment_msg;
ALTER TABLE script DROP COLUMN deployment_callbacks;