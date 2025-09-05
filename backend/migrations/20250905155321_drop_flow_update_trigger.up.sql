-- drop flow_update_trigger. We introduce new and more precise trigger in previous migration
DROP TRIGGER flow_update_trigger ON flow_version;
