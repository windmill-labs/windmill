-- Add down migration script here
DROP TRIGGER workspace_premium_change_trigger ON workspace;
DROP FUNCTION notify_workspace_premium_change();
