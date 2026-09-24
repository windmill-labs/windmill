DROP TRIGGER IF EXISTS script_list_change_update_trigger ON script;
DROP TRIGGER IF EXISTS script_list_change_delete_trigger ON script;
DROP TRIGGER IF EXISTS flow_list_change_update_trigger ON flow;
DROP TRIGGER IF EXISTS flow_list_change_delete_trigger ON flow;
DROP FUNCTION IF EXISTS notify_runnable_list_change();
