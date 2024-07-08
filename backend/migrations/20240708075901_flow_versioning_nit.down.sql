-- Add down migration script here
ALTER TABLE flow_version ALTER COLUMN value DROP NOT NULL;