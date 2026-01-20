-- Remove column for disabling error handler on u/ paths
ALTER TABLE workspace_settings DROP COLUMN IF EXISTS error_handler_muted_on_user_path;
