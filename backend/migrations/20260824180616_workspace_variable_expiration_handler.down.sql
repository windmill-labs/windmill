ALTER TABLE workspace_settings
    DROP COLUMN IF EXISTS variable_expiration_handler,
    DROP COLUMN IF EXISTS variable_expiration_failed_at;
