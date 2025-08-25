-- Add up migration script here
ALTER TABLE script ADD COLUMN ws_error_handler_muted BOOLEAN NOT NULL DEFAULT false;
ALTER TABLE flow ADD COLUMN ws_error_handler_muted BOOLEAN NOT NULL DEFAULT false;