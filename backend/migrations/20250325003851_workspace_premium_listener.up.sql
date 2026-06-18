-- Add up migration script here
CREATE OR REPLACE FUNCTION notify_workspace_premium_change()
RETURNS TRIGGER AS $$
BEGIN
    PERFORM pg_notify('notify_workspace_premium_change', NEW.id);
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER workspace_premium_change_trigger
AFTER UPDATE OF premium ON workspace
FOR EACH ROW
EXECUTE FUNCTION notify_workspace_premium_change();
