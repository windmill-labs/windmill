-- Add down migration script here
ALTER TABLE http_trigger DROP COLUMN error_handler_path, DROP COLUMN error_handler_args, DROP COLUMN retry;