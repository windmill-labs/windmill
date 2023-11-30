-- Add up migration script here
ALTER TYPE JOB_KIND ADD VALUE IF NOT EXISTS 'deploymentcallback';
ALTER TABLE workspace_settings ADD COLUMN IF NOT EXISTS git_sync JSONB;

ALTER TABLE script ADD COLUMN IF NOT EXISTS deployment_msg TEXT;
ALTER TABLE script ADD COLUMN IF NOT EXISTS deployment_callbacks UUID[];
