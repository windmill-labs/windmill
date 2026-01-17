-- Add success_handler columns to workspace_settings
ALTER TABLE workspace_settings
    ADD COLUMN IF NOT EXISTS success_handler TEXT,
    ADD COLUMN IF NOT EXISTS success_handler_extra_args JSON;
