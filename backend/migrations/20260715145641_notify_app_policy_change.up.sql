-- Emit a notify_event so every server evicts its cached app policy
-- (windmill-api APP_POLICY_CACHE) when the policy changes or the app is deleted;
-- without it a revoked app stays anonymously executable from stale cache
-- (GHSA-r5v4-cxh9-7qhq). SECURITY DEFINER so the INSERT runs as the function
-- owner rather than the invoking windmill_user/windmill_admin role, matching the
-- other notify_* triggers.
CREATE OR REPLACE FUNCTION notify_app_policy_change()
RETURNS TRIGGER AS $$
BEGIN
    INSERT INTO notify_event (channel, payload)
    VALUES (
        'notify_app_policy_change',
        COALESCE(NEW.workspace_id, OLD.workspace_id) || ':' || COALESCE(NEW.path, OLD.path)
    );
    RETURN COALESCE(NEW, OLD);
END;
$$ LANGUAGE plpgsql SECURITY DEFINER;

CREATE TRIGGER app_policy_change_trigger
AFTER UPDATE OF policy ON app
FOR EACH ROW
WHEN (OLD.policy IS DISTINCT FROM NEW.policy)
EXECUTE FUNCTION notify_app_policy_change();

CREATE TRIGGER app_policy_delete_trigger
AFTER DELETE ON app
FOR EACH ROW
EXECUTE FUNCTION notify_app_policy_change();
