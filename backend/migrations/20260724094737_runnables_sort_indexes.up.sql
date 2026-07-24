-- Indexes backing the unified homepage runnables listing so the merged,
-- keyset-paginated query stays an index scan (Merge Append + LIMIT) even on big
-- workspaces, for both the time orders and the name orders.

-- Time orders. Scripts accumulate archived version rows, so scope that index to
-- the visible non-archived set the homepage browses.
CREATE INDEX IF NOT EXISTS index_script_on_workspace_created_at
    ON script (workspace_id, created_at DESC)
    WHERE archived = false;

CREATE INDEX IF NOT EXISTS index_flow_on_workspace_edited_at
    ON flow (workspace_id, edited_at DESC);

-- Name orders sort on the lowered summary-or-path expression, so a matching
-- expression index makes that order presorted instead of a full sort per page.
CREATE INDEX IF NOT EXISTS index_script_on_workspace_name
    ON script (workspace_id, lower(COALESCE(NULLIF(summary, ''), path)))
    WHERE archived = false;

CREATE INDEX IF NOT EXISTS index_flow_on_workspace_name
    ON flow (workspace_id, lower(COALESCE(NULLIF(summary, ''), path)));

CREATE INDEX IF NOT EXISTS index_app_on_workspace_name
    ON app (workspace_id, lower(COALESCE(NULLIF(summary, ''), path)));
