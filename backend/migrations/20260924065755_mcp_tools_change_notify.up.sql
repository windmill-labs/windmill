-- Deploys already emit `notify_runnable_version_change`; these cover the other changes to what
-- the MCP server lists as tools. One event per workspace per transaction:
-- archiving a path touches every version of it and deleting a workspace every script, while the
-- poll loop reads 1000 events per tick, so a row-level burst would delay every other channel.
CREATE OR REPLACE FUNCTION notify_mcp_tools_change()
RETURNS TRIGGER AS $$
DECLARE
    ws TEXT := COALESCE(NEW.workspace_id, OLD.workspace_id);
    seen TEXT := COALESCE(current_setting('windmill.mcp_tools_notified', true), '');
BEGIN
    IF position(',' || ws || ',' IN seen) = 0 THEN
        INSERT INTO notify_event (channel, payload) VALUES ('notify_mcp_tools_change', ws);
        PERFORM set_config('windmill.mcp_tools_notified', seen || ',' || ws || ',', true);
    END IF;
    RETURN NULL;
END;
$$ LANGUAGE plpgsql SECURITY DEFINER;

CREATE TRIGGER script_mcp_tools_update_trigger
AFTER UPDATE OF archived, deleted ON script
FOR EACH ROW
WHEN (OLD.archived IS DISTINCT FROM NEW.archived OR OLD.deleted IS DISTINCT FROM NEW.deleted)
EXECUTE FUNCTION notify_mcp_tools_change();

CREATE TRIGGER script_mcp_tools_delete_trigger
AFTER DELETE ON script
FOR EACH ROW
EXECUTE FUNCTION notify_mcp_tools_change();

-- A create inserts the row with no versions and appends one later, which the versions trigger
-- sees. Restore from trash, rename and fork insert the row with its versions already set.
CREATE TRIGGER flow_mcp_tools_insert_trigger
AFTER INSERT ON flow
FOR EACH ROW
WHEN (cardinality(NEW.versions) > 0)
EXECUTE FUNCTION notify_mcp_tools_change();

CREATE TRIGGER flow_mcp_tools_update_trigger
AFTER UPDATE OF archived ON flow
FOR EACH ROW
WHEN (OLD.archived IS DISTINCT FROM NEW.archived)
EXECUTE FUNCTION notify_mcp_tools_change();

CREATE TRIGGER flow_mcp_tools_delete_trigger
AFTER DELETE ON flow
FOR EACH ROW
EXECUTE FUNCTION notify_mcp_tools_change();

-- A tool's input schema lists the workspace's resources of each resource type it takes.
CREATE TRIGGER resource_mcp_tools_insert_delete_trigger
AFTER INSERT OR DELETE ON resource
FOR EACH ROW
EXECUTE FUNCTION notify_mcp_tools_change();

CREATE TRIGGER resource_mcp_tools_rename_trigger
AFTER UPDATE OF path ON resource
FOR EACH ROW
WHEN (OLD.path IS DISTINCT FROM NEW.path)
EXECUTE FUNCTION notify_mcp_tools_change();

-- Favorites are the tool list of an `mcp:favorites` token.
CREATE TRIGGER favorite_mcp_tools_insert_trigger
AFTER INSERT ON favorite
FOR EACH ROW
WHEN (NEW.favorite_kind IN ('script', 'flow'))
EXECUTE FUNCTION notify_mcp_tools_change();

CREATE TRIGGER favorite_mcp_tools_delete_trigger
AFTER DELETE ON favorite
FOR EACH ROW
WHEN (OLD.favorite_kind IN ('script', 'flow'))
EXECUTE FUNCTION notify_mcp_tools_change();
