DROP TRIGGER IF EXISTS script_insert_trigger ON script;

CREATE TRIGGER script_insert_trigger
AFTER INSERT ON script
FOR EACH ROW
WHEN (NEW.lock IS NOT NULL)
EXECUTE FUNCTION notify_runnable_version_change('script');
