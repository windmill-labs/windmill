-- Indexes backing the unified homepage runnables listing so the merged,
-- keyset-paginated query stays an index scan (Merge Append + LIMIT) even on big
-- workspaces, for both the time orders and the name orders.
--
-- `archived` is the second key so the default (archived = false) and the
-- "Only archived" views each seek their own slice and still get the sort key
-- ordered within it, instead of one view scanning past the other's rows. Apps
-- have no archived column and are excluded from the archived view.

-- Time orders.
CREATE INDEX IF NOT EXISTS index_script_on_workspace_created_at
    ON script (workspace_id, archived, created_at DESC);

CREATE INDEX IF NOT EXISTS index_flow_on_workspace_edited_at
    ON flow (workspace_id, archived, edited_at DESC);

-- Name orders sort on the lowered summary-or-path expression, so a matching
-- expression index makes that order presorted instead of a full sort per page.
CREATE INDEX IF NOT EXISTS index_script_on_workspace_name
    ON script (workspace_id, archived, lower(COALESCE(NULLIF(summary, ''), path)));

CREATE INDEX IF NOT EXISTS index_flow_on_workspace_name
    ON flow (workspace_id, archived, lower(COALESCE(NULLIF(summary, ''), path)));

CREATE INDEX IF NOT EXISTS index_app_on_workspace_name
    ON app (workspace_id, lower(COALESCE(NULLIF(summary, ''), path)));
