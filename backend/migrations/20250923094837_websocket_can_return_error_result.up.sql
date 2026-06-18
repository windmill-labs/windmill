-- Add up migration script here
ALTER TABLE websocket_trigger ADD COLUMN can_return_error_result BOOLEAN NOT NULL DEFAULT FALSE;