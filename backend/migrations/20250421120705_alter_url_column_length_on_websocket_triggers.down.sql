-- Add down migration script here
ALTER TABLE websocket_trigger
ALTER COLUMN url TYPE VARCHAR(255);