-- Add up migration script here
ALTER TABLE instance_group
RENAME COLUMN external_id TO id;
ALTER TABLE instance_group
ADD COLUMN external_id VARCHAR(512);