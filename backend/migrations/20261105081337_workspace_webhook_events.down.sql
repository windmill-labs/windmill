DROP TRIGGER IF EXISTS webhook_change_trigger ON workspace_settings;
CREATE TRIGGER webhook_change_trigger
AFTER UPDATE OF webhook ON workspace_settings
FOR EACH ROW
WHEN (OLD.webhook IS DISTINCT FROM NEW.webhook)
EXECUTE FUNCTION notify_webhook_change();

ALTER TABLE workspace_settings DROP COLUMN IF EXISTS webhook_events;
