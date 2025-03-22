-- Add down migration script here
ALTER TABLE http_trigger
DROP COLUMN authentication_resource_path;
