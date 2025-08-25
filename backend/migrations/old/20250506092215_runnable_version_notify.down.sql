-- Add down migration script here
DROP TRIGGER script_update_trigger ON script;
DROP TRIGGER flow_update_trigger ON flow_version;
DROP FUNCTION notify_runnable_version_change();
