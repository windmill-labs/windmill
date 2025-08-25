-- Add up migration script here
ALTER TABLE workspace_settings ADD COLUMN error_handler_extra_args JSON;