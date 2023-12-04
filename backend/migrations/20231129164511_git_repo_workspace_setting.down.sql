-- Add down migration script here
ALTER TABLE workspace_settings DROP COLUMN git_sync;

DROP TABLE IF EXISTS deployment_metadata;
