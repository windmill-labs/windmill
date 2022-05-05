-- Add down migration script here
ALTER TABLE script
ALTER COLUMN schema TYPE jsonb;

ALTER TABLE flow
ALTER COLUMN schema TYPE jsonb;
