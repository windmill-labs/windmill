-- Add auto-add columns for instance groups
ALTER TABLE workspace_settings
ADD COLUMN auto_add_instance_groups text[] DEFAULT '{}',
ADD COLUMN auto_add_instance_groups_roles jsonb DEFAULT '{}';
