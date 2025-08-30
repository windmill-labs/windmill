-- Add workspace key cache invalidation trigger

CREATE OR REPLACE FUNCTION notify_workspace_key_change()
RETURNS TRIGGER AS $$
BEGIN
    IF TG_OP = 'DELETE' THEN
        PERFORM pg_notify('notify_workspace_key_change', OLD.workspace_id);
        RETURN OLD;
    ELSE
        PERFORM pg_notify('notify_workspace_key_change', NEW.workspace_id);
        RETURN NEW;
    END IF;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER workspace_key_change_trigger
AFTER INSERT OR UPDATE OF key OR DELETE ON workspace_key
FOR EACH ROW
EXECUTE FUNCTION notify_workspace_key_change();