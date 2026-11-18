-- NULL sends every event; an array sends only the event types it lists.
ALTER TABLE workspace_settings ADD COLUMN IF NOT EXISTS webhook_events TEXT[];

DROP TRIGGER IF EXISTS webhook_change_trigger ON workspace_settings;
CREATE TRIGGER webhook_change_trigger
AFTER UPDATE OF webhook, webhook_events ON workspace_settings
FOR EACH ROW
WHEN (OLD.webhook IS DISTINCT FROM NEW.webhook OR OLD.webhook_events IS DISTINCT FROM NEW.webhook_events)
EXECUTE FUNCTION notify_webhook_change();
