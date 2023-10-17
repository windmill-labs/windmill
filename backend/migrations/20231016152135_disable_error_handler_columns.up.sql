-- Add up migration script here
ALTER TABLE script ADD COLUMN ws_error_handler_enabled BOOLEAN NOT NULL DEFAULT true;
ALTER TABLE queue ADD COLUMN ws_error_handler_enabled BOOLEAN NOT NULL DEFAULT true;