-- A deploy changes what the cached entry (windmill-api APP_POLICY_CACHE) holds even
-- when the policy is byte-identical: the cache also carries the context inputs the
-- deployed runnables declare, read from the app value. The version in an execute
-- request is the caller's claim, so it cannot be what evicts a stale entry here.
DROP TRIGGER IF EXISTS app_policy_change_trigger ON app;

CREATE TRIGGER app_policy_change_trigger
AFTER UPDATE OF policy, versions ON app
FOR EACH ROW
WHEN (OLD.policy IS DISTINCT FROM NEW.policy OR OLD.versions IS DISTINCT FROM NEW.versions)
EXECUTE FUNCTION notify_app_policy_change();
