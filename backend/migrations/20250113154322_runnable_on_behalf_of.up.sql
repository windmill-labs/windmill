-- Add up migration script here
ALTER TABLE script ADD COLUMN on_behalf_of_email TEXT;
ALTER TABLE flow ADD COLUMN on_behalf_of_email TEXT;