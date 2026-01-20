-- Add column to disable error handler for scripts/flows starting with u/
ALTER TABLE workspace_settings ADD COLUMN IF NOT EXISTS error_handler_muted_on_user_path BOOL NOT NULL DEFAULT false;
