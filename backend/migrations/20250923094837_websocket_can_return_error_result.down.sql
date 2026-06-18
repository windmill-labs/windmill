-- Add down migration script here
ALTER TABLE websocket_trigger DROP COLUMN can_return_error_result;