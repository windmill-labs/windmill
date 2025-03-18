-- Add up migration script here
ALTER TABLE http_trigger
ADD COLUMN webhook_auth jsonb DEFAULT NULL;