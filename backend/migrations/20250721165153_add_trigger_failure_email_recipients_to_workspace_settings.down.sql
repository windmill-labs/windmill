-- Add down migration script here
ALTER TABLE workspace_settings
    DROP COLUMN IF EXISTS email_recipients;