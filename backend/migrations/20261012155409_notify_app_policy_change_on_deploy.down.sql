DROP TRIGGER IF EXISTS app_policy_change_trigger ON app;

CREATE TRIGGER app_policy_change_trigger
AFTER UPDATE OF policy ON app
FOR EACH ROW
WHEN (OLD.policy IS DISTINCT FROM NEW.policy)
EXECUTE FUNCTION notify_app_policy_change();
