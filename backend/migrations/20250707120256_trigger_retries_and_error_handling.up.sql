-- Add up migration script here
ALTER TABLE http_trigger ADD COLUMN error_handler_path TEXT, ADD COLUMN error_handler_args JSONB, ADD COLUMN retry JSONB;