-- Add down migration script here
ALTER TABLE http_trigger
DROP COLUMN webhook_resource_path;