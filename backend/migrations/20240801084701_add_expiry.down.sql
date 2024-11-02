-- Add down migration script here
ALTER TABLE variable DROP COLUMN IF EXISTS expires_at;