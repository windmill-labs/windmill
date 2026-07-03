-- Captured output schema of a managed `// materialize` asset (gap #2a).
-- After a managed materialize commits, the worker DESCRIBEs the written table
-- and records its column list here as asset-level metadata. Schema is a
-- property of the asset/table, not of a partition slice, so it lives in its own
-- table keyed by (workspace, asset_kind, asset_path) rather than as a column on
-- materialized_partition (which would duplicate the identical schema across
-- every partition row). This is the producer-side capture that #2b (save-time
-- consumer-ref contract enforcement) reads back.
--
-- Versioning across re-materializations: a new `version` row is inserted only
-- when the captured column set differs from the latest stored version for the
-- asset; an unchanged re-materialize re-affirms the latest row in place. So the
-- table is a compact schema-evolution history and MAX(version) is the current
-- contract.
CREATE TABLE IF NOT EXISTS materialized_asset_schema (
  workspace_id  VARCHAR(50) NOT NULL REFERENCES workspace(id) ON DELETE CASCADE ON UPDATE CASCADE,
  asset_kind    ASSET_KIND NOT NULL,
  asset_path    VARCHAR(255) NOT NULL,
  -- Monotonic per (workspace, asset_kind, asset_path), starting at 1; only
  -- bumped when the schema actually changes.
  version       BIGINT NOT NULL,
  -- The captured columns, ordered as the table presents them:
  -- [{"name": "...", "type": "..."}, ...].
  columns       JSONB NOT NULL,
  -- DuckLake snapshot the schema was captured from (NULL for non-ducklake /
  -- substrates without snapshots).
  snapshot_id   BIGINT,
  job_id        UUID,
  captured_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
  PRIMARY KEY (workspace_id, asset_kind, asset_path, version)
);

-- Default privileges (migration 20250205131523) only apply to objects created
-- by the role that set them, so grant explicitly — the API reads/writes this
-- table as the invoking role (same fix as script_trigger in 20260619112847).
GRANT ALL ON materialized_asset_schema TO windmill_user;
GRANT ALL ON materialized_asset_schema TO windmill_admin;
