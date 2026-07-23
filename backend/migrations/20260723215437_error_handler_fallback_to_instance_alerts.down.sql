-- Add down migration script here
ALTER TABLE workspace_settings
    DROP COLUMN IF EXISTS error_handler_fallback_to_instance_alerts;
