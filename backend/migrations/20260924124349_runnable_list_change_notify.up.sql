-- Signals that a workspace's set of live scripts/flows shrank (archive, delete, and flow
-- renames, which delete the old row). Deploys are already signalled by
-- `notify_runnable_version_change`. Statement-level so that a bulk change, such as deleting a
-- workspace or a user's items, emits one event per workspace rather than one per row.
CREATE OR REPLACE FUNCTION notify_runnable_list_change()
RETURNS TRIGGER AS $$
BEGIN
    IF TG_OP = 'DELETE' THEN
        INSERT INTO notify_event (channel, payload)
        SELECT DISTINCT 'notify_runnable_list_change', workspace_id FROM old_rows;
    ELSIF TG_TABLE_NAME = 'script' THEN
        INSERT INTO notify_event (channel, payload)
        SELECT DISTINCT 'notify_runnable_list_change', n.workspace_id
        FROM new_rows n
        JOIN old_rows o ON o.workspace_id = n.workspace_id AND o.hash = n.hash
        WHERE o.archived IS DISTINCT FROM n.archived OR o.deleted IS DISTINCT FROM n.deleted;
    ELSE
        INSERT INTO notify_event (channel, payload)
        SELECT DISTINCT 'notify_runnable_list_change', n.workspace_id
        FROM new_rows n
        JOIN old_rows o ON o.workspace_id = n.workspace_id AND o.path = n.path
        WHERE o.archived IS DISTINCT FROM n.archived;
    END IF;
    RETURN NULL;
END;
$$ LANGUAGE plpgsql SECURITY DEFINER;

-- Postgres rejects transition tables on a trigger with a column list, so the UPDATE triggers
-- fire on every UPDATE statement and the function filters for the columns that matter.
CREATE TRIGGER script_list_change_update_trigger
AFTER UPDATE ON script
REFERENCING OLD TABLE AS old_rows NEW TABLE AS new_rows
FOR EACH STATEMENT
EXECUTE FUNCTION notify_runnable_list_change();

CREATE TRIGGER script_list_change_delete_trigger
AFTER DELETE ON script
REFERENCING OLD TABLE AS old_rows
FOR EACH STATEMENT
EXECUTE FUNCTION notify_runnable_list_change();

CREATE TRIGGER flow_list_change_update_trigger
AFTER UPDATE ON flow
REFERENCING OLD TABLE AS old_rows NEW TABLE AS new_rows
FOR EACH STATEMENT
EXECUTE FUNCTION notify_runnable_list_change();

CREATE TRIGGER flow_list_change_delete_trigger
AFTER DELETE ON flow
REFERENCING OLD TABLE AS old_rows
FOR EACH STATEMENT
EXECUTE FUNCTION notify_runnable_list_change();
