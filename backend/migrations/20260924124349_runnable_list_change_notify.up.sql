-- Signals that a workspace's set of live scripts/flows changed other than by a deploy, which
-- `notify_runnable_version_change` already signals: an archive, a delete, a flow rename (which
-- deletes the old row), or a path move such as a username change or an offboarding.

-- Row-level with a column list and a WHEN guard, so the many unrelated UPDATEs on these tables
-- (locks, `on_behalf_of` rewrites, workspace renames) pay nothing. A statement-level trigger would
-- need transition tables, which Postgres refuses to combine with a column list, and would copy
-- every updated row, content included. A path move signals only unarchived rows: a bulk move
-- (username change, offboarding) also rewrites every archived version, which no listing shows.
CREATE OR REPLACE FUNCTION notify_runnable_list_change()
RETURNS TRIGGER AS $$
BEGIN
    INSERT INTO notify_event (channel, payload)
    VALUES ('notify_runnable_list_change', NEW.workspace_id);
    RETURN NULL;
END;
$$ LANGUAGE plpgsql SECURITY DEFINER;

-- Statement-level, so deleting a workspace or a user's items emits one event per workspace
-- rather than one per row.
CREATE OR REPLACE FUNCTION notify_runnable_list_delete()
RETURNS TRIGGER AS $$
BEGIN
    INSERT INTO notify_event (channel, payload)
    SELECT DISTINCT 'notify_runnable_list_change', workspace_id FROM old_rows;
    RETURN NULL;
END;
$$ LANGUAGE plpgsql SECURITY DEFINER;

CREATE TRIGGER script_list_change_update_trigger
AFTER UPDATE OF archived, deleted, path ON script
FOR EACH ROW
WHEN (OLD.archived IS DISTINCT FROM NEW.archived
    OR OLD.deleted IS DISTINCT FROM NEW.deleted
    OR (OLD.path IS DISTINCT FROM NEW.path AND NOT NEW.archived))
EXECUTE FUNCTION notify_runnable_list_change();

CREATE TRIGGER script_list_change_delete_trigger
AFTER DELETE ON script
REFERENCING OLD TABLE AS old_rows
FOR EACH STATEMENT
EXECUTE FUNCTION notify_runnable_list_delete();

CREATE TRIGGER flow_list_change_update_trigger
AFTER UPDATE OF archived, path ON flow
FOR EACH ROW
WHEN (OLD.archived IS DISTINCT FROM NEW.archived
    OR (OLD.path IS DISTINCT FROM NEW.path AND NOT NEW.archived))
EXECUTE FUNCTION notify_runnable_list_change();

CREATE TRIGGER flow_list_change_delete_trigger
AFTER DELETE ON flow
REFERENCING OLD TABLE AS old_rows
FOR EACH STATEMENT
EXECUTE FUNCTION notify_runnable_list_delete();
