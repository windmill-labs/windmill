-- Trigger cache invalidation whenever the .versions are updated in `flow`
-- This is ultimate way of instant cache invalidation. If the notification is being lost, lazy expiration timer will work as a fallback
CREATE TRIGGER flow_versions_append_trigger
AFTER UPDATE ON flow
FOR EACH ROW
-- Trigger only when versions are different
-- TODO: PERF: Compare by lengths?
WHEN (NEW.versions <> OLD.versions)
EXECUTE FUNCTION notify_runnable_version_change('flow');
