-- Add up migration script here
ALTER TABLE workspace_settings
    ADD COLUMN email_recipients TEXT[] DEFAULT NULL;