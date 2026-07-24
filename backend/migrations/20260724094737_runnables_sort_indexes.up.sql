-- Indexes backing the unified homepage runnables listing when ordered by
-- last-updated time. The name orders already ride the existing
-- (workspace_id, path) indexes (script index_script_on_path_created_at,
-- flow_pkey, app unique_path_workspace_id); apps are one row per path and few
-- enough to sort without a dedicated index. These two cover the large tables so
-- the merged, keyset-paginated query stays an index scan even on big workspaces.

-- Scripts accumulate archived version rows, so scope the index to the visible
-- (non-archived) set the homepage browses.
CREATE INDEX IF NOT EXISTS index_script_on_workspace_created_at
    ON script (workspace_id, created_at DESC)
    WHERE archived = false;

CREATE INDEX IF NOT EXISTS index_flow_on_workspace_edited_at
    ON flow (workspace_id, edited_at DESC);
