-- Add down migration script here
ALTER TABLE websocket_trigger DROP COLUMN initial_messages, DROP COLUMN url_runnable_args;