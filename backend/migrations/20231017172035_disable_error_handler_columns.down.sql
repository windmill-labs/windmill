-- Add down migration script here
ALTER TABLE script DROP COLUMN ws_error_handler_muted;
ALTER TABLE flow DROP COLUMN ws_error_handler_muted;