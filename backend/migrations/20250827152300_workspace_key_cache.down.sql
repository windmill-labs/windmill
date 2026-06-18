-- Remove workspace key cache invalidation trigger

DROP TRIGGER workspace_key_change_trigger ON workspace_key;
DROP FUNCTION notify_workspace_key_change();