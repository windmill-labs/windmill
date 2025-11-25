-- Add up migration script here
ALTER TABLE http_trigger ADD COLUMN enabled BOOLEAN DEFAULT TRUE NOT NULL;