-- Add down migration script here
ALTER TABLE instance_group
DROP COLUMN external_id;
ALTER TABLE instance_group
RENAME COLUMN id TO external_id;