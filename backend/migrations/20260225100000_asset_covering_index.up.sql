-- Covering index for the list_assets CTE: GROUP BY (path, kind) + MAX(created_at, id) + ORDER BY
-- Includes usage_kind and usage_path to allow full index-only scan (avoiding heap lookups for filter conditions)
CREATE INDEX IF NOT EXISTS idx_asset_ws_path_kind_recent
    ON asset (workspace_id, path, kind, created_at DESC, id DESC)
    INCLUDE (usage_kind, usage_path);

-- Drop indexes now subsumed by idx_asset_ws_path_kind_recent:
-- idx_asset_workspace_created_id (workspace_id, created_at DESC, id DESC) - only used by list_assets CTE
-- idx_asset_kind_path (workspace_id, kind, path) - only used by list_assets CTE/outer join, covered by new index + PK
DROP INDEX IF EXISTS idx_asset_workspace_created_id;
DROP INDEX IF EXISTS idx_asset_kind_path;
