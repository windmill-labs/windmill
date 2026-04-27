DROP TABLE IF EXISTS script_trigger;
DROP TYPE IF EXISTS SCRIPT_TRIGGER_KIND;

-- Recreate the materializer tables so rolling back to the pre-refactor
-- backend code boots. Data is gone either way (irrecoverable).
CREATE TABLE asset_materializer (
  workspace_id VARCHAR(50) NOT NULL REFERENCES workspace(id) ON DELETE CASCADE ON UPDATE CASCADE,
  asset_kind ASSET_KIND NOT NULL,
  asset_path VARCHAR(255) NOT NULL,
  runnable_kind ASSET_USAGE_KIND NOT NULL,
  runnable_path VARCHAR(255) NOT NULL,
  deployed_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  deployed_by VARCHAR(50) NOT NULL,
  PRIMARY KEY (workspace_id, asset_kind, asset_path)
);
CREATE INDEX idx_asset_materializer_runnable
  ON asset_materializer (workspace_id, runnable_kind, runnable_path);

CREATE TABLE asset_materializer_history (
  id BIGSERIAL PRIMARY KEY,
  workspace_id VARCHAR(50) NOT NULL REFERENCES workspace(id) ON DELETE CASCADE ON UPDATE CASCADE,
  asset_kind ASSET_KIND NOT NULL,
  asset_path VARCHAR(255) NOT NULL,
  runnable_kind ASSET_USAGE_KIND NOT NULL,
  runnable_path VARCHAR(255) NOT NULL,
  deployed_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  deployed_by VARCHAR(50) NOT NULL
);
CREATE INDEX idx_asset_materializer_history_asset
  ON asset_materializer_history (workspace_id, asset_kind, asset_path, deployed_at DESC);
