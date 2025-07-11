CREATE TYPE ASSET_USAGE_KIND AS ENUM ('script', 'flow');
CREATE TYPE ASSET_ACCESS_TYPE AS ENUM ('r', 'w', 'rw');
CREATE TYPE ASSET_KIND AS ENUM ('s3object', 'resource', 'variable');

CREATE TABLE asset (
  workspace_id VARCHAR(50) NOT NULL REFERENCES workspace(id) ON DELETE CASCADE ON UPDATE CASCADE,
  path VARCHAR(255) NOT NULL,
  kind ASSET_KIND NOT NULL,
  usage_access_type ASSET_ACCESS_TYPE,
  usage_path VARCHAR(255) NOT NULL,
  usage_kind ASSET_USAGE_KIND NOT NULL,
  PRIMARY KEY (workspace_id, path, kind, usage_path, usage_kind)
);

CREATE INDEX idx_asset_usage ON asset (workspace_id, usage_path, usage_kind);
CREATE INDEX idx_asset_kind_path ON asset (workspace_id, kind, path);