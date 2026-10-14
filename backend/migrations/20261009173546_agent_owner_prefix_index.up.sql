-- Backs the agent branch of /runnables/list and /runnables/counts, which reach the
-- `ai_agent` rows of `resource` by workspace and byte-ordered path prefix. Partial, so
-- neither walks the workspace's other resources (state, cache, ...).
-- Created CONCURRENTLY via the OVERRIDDEN_MIGRATIONS rewrite in windmill-api/src/db.rs.
CREATE INDEX IF NOT EXISTS idx_resource_agent_owner_prefix
    ON resource (workspace_id, path text_pattern_ops)
    WHERE resource_type = 'ai_agent';
