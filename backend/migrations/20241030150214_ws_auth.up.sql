-- Add up migration script here
ALTER TABLE websocket_trigger ADD COLUMN initial_messages JSONB[] DEFAULT '{}', ADD COLUMN url_runnable_args JSONB DEFAULT '{}';