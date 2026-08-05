-- Add down migration script here
ALTER TABLE script DROP COLUMN IF EXISTS on_behalf_of;
ALTER TABLE flow DROP COLUMN IF EXISTS on_behalf_of;
