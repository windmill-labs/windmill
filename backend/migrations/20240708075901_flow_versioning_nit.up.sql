-- Add up migration script here
UPDATE flow_version SET value = '{"modules":[]}'::jsonb WHERE value IS NULL;
ALTER TABLE flow_version ALTER COLUMN value SET NOT NULL;