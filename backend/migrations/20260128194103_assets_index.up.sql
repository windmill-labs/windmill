-- Postgres requires indexes to be created in a separate migration (transaction) after columns are added.

-- Index for pagination queries that use workspace_id, created_at, and id for cursor pagination
-- Supports: SELECT with GROUP BY path, kind ORDER BY MAX(created_at) DESC, MAX(id) DESC
CREATE INDEX idx_asset_workspace_created_id ON asset (workspace_id, created_at DESC, id DESC);

-- Filtered index for job pruning operations that delete old job assets
-- Supports: DELETE queries with WHERE usage_kind = 'job' and window functions on (workspace_id, path, kind) ORDER BY created_at DESC
CREATE INDEX idx_asset_job_pruning ON asset (workspace_id, path, kind, created_at DESC) WHERE usage_kind = 'job';
