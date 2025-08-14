-- Remove auto-add columns for instance groups
ALTER TABLE workspace_settings
DROP COLUMN auto_add_instance_groups,
DROP COLUMN auto_add_instance_groups_roles;
