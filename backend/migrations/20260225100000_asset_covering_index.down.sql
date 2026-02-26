DROP INDEX IF EXISTS idx_asset_ws_path_kind_recent;

-- Restore the dropped indexes
CREATE INDEX IF NOT EXISTS idx_asset_workspace_created_id ON asset (workspace_id, created_at DESC, id DESC);
CREATE INDEX IF NOT EXISTS idx_asset_kind_path ON asset (workspace_id, kind, path);
