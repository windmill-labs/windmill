-- Add up migration script here
ALTER TABLE workspace_settings ADD COLUMN IF NOT EXISTS error_handler_muted_on_cancel BOOL NOT NULL DEFAULT false;