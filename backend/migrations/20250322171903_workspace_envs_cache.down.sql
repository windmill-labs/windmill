-- Add down migration script here
DROP TRIGGER workspace_envs_change_trigger ON workspace_env;
DROP FUNCTION notify_workspace_envs_change();
