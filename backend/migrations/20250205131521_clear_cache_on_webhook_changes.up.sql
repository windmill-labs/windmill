-- Add up migration script here

CREATE OR REPLACE FUNCTION notify_webhook_change()
RETURNS TRIGGER AS $$
BEGIN
    PERFORM pg_notify('notify_webhook_change', NEW.workspace_id);
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER webhook_change_trigger
AFTER UPDATE OF webhook ON workspace_settings
FOR EACH ROW
WHEN (OLD.webhook IS DISTINCT FROM NEW.webhook)
EXECUTE FUNCTION notify_webhook_change();
