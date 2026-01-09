-- Add down migration script here
ALTER TABLE script DROP COLUMN IF EXISTS debounce_key;
ALTER TABLE script DROP COLUMN IF EXISTS debounce_delay_s;

