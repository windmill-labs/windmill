-- A version deployed without a lockfile gets one later from a dependency job, so gating the
-- insert notification on `lock IS NOT NULL` left the path-to-hash caches holding the previous
-- version until that UPDATE (or their TTL) came around. Notify on every insert instead: the
-- deploy itself is what makes the cached answer suspect.
DROP TRIGGER IF EXISTS script_insert_trigger ON script;

CREATE TRIGGER script_insert_trigger
AFTER INSERT ON script
FOR EACH ROW
EXECUTE FUNCTION notify_runnable_version_change('script');
