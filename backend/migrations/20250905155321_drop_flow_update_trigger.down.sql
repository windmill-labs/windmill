-- Add down migration script here
CREATE TRIGGER flow_update_trigger
AFTER INSERT ON flow_version
FOR EACH ROW
EXECUTE FUNCTION notify_runnable_version_change('flow');

