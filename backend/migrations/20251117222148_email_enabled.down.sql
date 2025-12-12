-- Add down migration script here
ALTER TABLE email_trigger DROP COLUMN enabled;
