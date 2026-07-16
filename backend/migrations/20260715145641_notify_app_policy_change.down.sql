DROP TRIGGER IF EXISTS app_policy_delete_trigger ON app;
DROP TRIGGER IF EXISTS app_policy_change_trigger ON app;
DROP FUNCTION IF EXISTS notify_app_policy_change();
