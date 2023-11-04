-- Add up migration script here
ALTER TABLE schedule ADD COLUMN ws_error_handler_muted BOOLEAN NOT NULL DEFAULT false;
