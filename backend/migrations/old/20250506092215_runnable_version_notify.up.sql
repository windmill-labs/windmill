-- Add up migration script here
CREATE OR REPLACE FUNCTION notify_runnable_version_change()
RETURNS TRIGGER AS $$
DECLARE
    source_type TEXT;
BEGIN
    source_type := TG_ARGV[0];

    PERFORM pg_notify('notify_runnable_version_change', NEW.workspace_id || ':' || source_type || ':' || NEW.path);
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER script_update_trigger
AFTER UPDATE OF lock ON script
FOR EACH ROW
EXECUTE FUNCTION notify_runnable_version_change('script');

CREATE TRIGGER flow_update_trigger
AFTER INSERT ON flow_version
FOR EACH ROW
EXECUTE FUNCTION notify_runnable_version_change('flow');
