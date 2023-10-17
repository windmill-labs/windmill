-- Add down migration script here
ALTER TABLE script DROP COLUMN ws_error_handler_muted;
ALTER TABLE queue DROP COLUMN ws_error_handler_muted;