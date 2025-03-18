-- Add up migration script here
ALTER TABLE http_trigger
ADD COLUMN webhook_auth_resource_path VARCHAR(255) DEFAULT NULL;