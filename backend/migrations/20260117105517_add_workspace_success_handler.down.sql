-- Remove success_handler columns from workspace_settings
ALTER TABLE workspace_settings
    DROP COLUMN IF EXISTS success_handler,
    DROP COLUMN IF EXISTS success_handler_extra_args;
