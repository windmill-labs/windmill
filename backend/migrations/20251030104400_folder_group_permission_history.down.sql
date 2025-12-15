-- Add down migration script here
DROP INDEX IF EXISTS idx_group_perm_history_workspace_group;
DROP TABLE IF EXISTS group_permission_history;

DROP INDEX IF EXISTS idx_folder_perm_history_workspace_folder;
DROP TABLE IF EXISTS folder_permission_history;
