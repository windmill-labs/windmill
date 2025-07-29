-- Add up migration script here
ALTER TABLE workspace_settings
ADD COLUMN trigger_failure_email_recipients TEXT[] DEFAULT NULL;