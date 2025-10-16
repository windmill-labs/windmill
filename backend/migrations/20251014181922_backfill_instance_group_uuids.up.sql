-- Add up migration script here

-- Backfill UUIDs for instance groups that don't have one
-- This is needed for SCIM compatibility where groups must have stable UUIDs
UPDATE instance_group
SET id = gen_random_uuid()::text
WHERE id IS NULL;
