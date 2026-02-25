-- Covering index for the list_assets CTE: GROUP BY (path, kind) + MAX(created_at, id) + ORDER BY
-- Includes usage_kind and usage_path to allow full index-only scan (avoiding heap lookups for filter conditions)
CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_asset_ws_path_kind_recent
    ON asset (workspace_id, path, kind, created_at DESC, id DESC)
    INCLUDE (usage_kind, usage_path);
