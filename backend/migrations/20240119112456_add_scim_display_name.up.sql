-- Add up migration script here
ALTER TABLE instance_group ADD COLUMN IF NOT EXISTS scim_display_name VARCHAR(255);
UPDATE instance_group SET external_id = name, scim_display_name = name;