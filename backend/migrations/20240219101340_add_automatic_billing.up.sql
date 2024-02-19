-- Add up migration script here
ALTER TABLE workspace_settings
ADD COLUMN automatic_billing BOOLEAN NOT NULL DEFAULT FALSE;