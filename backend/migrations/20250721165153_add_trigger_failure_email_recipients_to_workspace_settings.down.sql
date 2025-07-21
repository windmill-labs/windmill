-- Add down migration script here
-- Add up migration script here
ALTER TABLE workspace_settings
DROP COLUMN IF EXISTS trigger_failure_email_recipients;