-- Add down migration script here
DROP TRIGGER http_trigger_change_trigger ON http_trigger;
DROP FUNCTION notify_http_trigger_change();
DROP SEQUENCE http_trigger_version_seq;