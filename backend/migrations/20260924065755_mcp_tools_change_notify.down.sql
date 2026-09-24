DROP TRIGGER IF EXISTS script_mcp_tools_update_trigger ON script;
DROP TRIGGER IF EXISTS script_mcp_tools_delete_trigger ON script;
DROP TRIGGER IF EXISTS flow_mcp_tools_update_trigger ON flow;
DROP TRIGGER IF EXISTS flow_mcp_tools_delete_trigger ON flow;
DROP TRIGGER IF EXISTS favorite_mcp_tools_insert_trigger ON favorite;
DROP TRIGGER IF EXISTS favorite_mcp_tools_delete_trigger ON favorite;
DROP FUNCTION IF EXISTS notify_mcp_tools_change();
