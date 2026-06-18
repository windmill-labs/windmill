-- Add up migration script here

ALTER TABLE email_trigger ADD COLUMN enabled BOOLEAN DEFAULT TRUE NOT NULL;