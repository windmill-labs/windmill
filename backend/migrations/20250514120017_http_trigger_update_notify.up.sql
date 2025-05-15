-- Add up migration script here
CREATE OR REPLACE FUNCTION notify_http_trigger_change()
RETURNS TRIGGER AS $$
BEGIN
    PERFORM pg_notify('notify_http_trigger_change', NEW.workspace_id || ':' || NEW.path);
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;
CREATE TRIGGER http_trigger_change_trigger
AFTER INSERT OR UPDATE OR DELETE ON http_trigger
FOR EACH ROW
EXECUTE FUNCTION notify_http_trigger_change();

CREATE SEQUENCE http_trigger_version_seq;

