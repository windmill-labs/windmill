-- Indexes backing the unified homepage runnables listing so the merged,
-- keyset-paginated query stays an index scan (Merge Append + LIMIT) even on big
-- workspaces, for both the time orders and the name orders.

-- Time orders. Not scoped to archived = false: the endpoint also serves the
-- "Only archived" view, and current (non-archived) versions carry the newest
-- created_at so they cluster at the head of the index for the default browse
-- anyway, while the archived view still gets an ordered index scan.
CREATE INDEX IF NOT EXISTS index_script_on_workspace_created_at
    ON script (workspace_id, created_at DESC);

CREATE INDEX IF NOT EXISTS index_flow_on_workspace_edited_at
    ON flow (workspace_id, edited_at DESC);

-- Name orders sort on the lowered summary-or-path expression, so a matching
-- expression index makes that order presorted instead of a full sort per page
-- (covers both the default and archived views).
CREATE INDEX IF NOT EXISTS index_script_on_workspace_name
    ON script (workspace_id, lower(COALESCE(NULLIF(summary, ''), path)));

CREATE INDEX IF NOT EXISTS index_flow_on_workspace_name
    ON flow (workspace_id, lower(COALESCE(NULLIF(summary, ''), path)));

CREATE INDEX IF NOT EXISTS index_app_on_workspace_name
    ON app (workspace_id, lower(COALESCE(NULLIF(summary, ''), path)));
