-- Add down migration script here
ALTER TABLE schedule DROP COLUMN ws_error_handler_muted;
