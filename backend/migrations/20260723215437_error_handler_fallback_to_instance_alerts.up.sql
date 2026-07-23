-- Add up migration script here
ALTER TABLE workspace_settings
    ADD COLUMN IF NOT EXISTS error_handler_fallback_to_instance_alerts BOOLEAN NOT NULL DEFAULT false;
