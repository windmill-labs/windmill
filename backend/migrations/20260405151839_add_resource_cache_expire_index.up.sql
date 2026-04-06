-- Partial functional index to speed up periodic cleanup of expired cache resources.
-- Matches the query: DELETE FROM resource WHERE resource_type = 'cache' AND to_timestamp((value->>'expire')::int) < now()
CREATE INDEX IF NOT EXISTS idx_resource_cache_expire
    ON resource (to_timestamp((value->>'expire')::int))
    WHERE resource_type = 'cache';
