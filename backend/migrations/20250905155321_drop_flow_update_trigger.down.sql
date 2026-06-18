-- Copied from runnable_version_notify.up.sql
CREATE TRIGGER flow_update_trigger
AFTER INSERT ON flow_version
FOR EACH ROW
EXECUTE FUNCTION notify_runnable_version_change('flow');

