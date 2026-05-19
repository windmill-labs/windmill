-- Current materializer: exactly one runnable owns each (workspace, kind, path) asset.
-- Updated via last-deploy-wins on `// materialize <path>` annotations.
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

-- Append-only history. Every claim (including takeovers) gets a row.
-- Revocations (redeploy without annotation) are represented by absence in
-- `asset_materializer` rather than a row here, so "who last materialized X"
-- is the most recent history row for X regardless of current state.
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
