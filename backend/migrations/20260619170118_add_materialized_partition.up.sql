-- Per-partition materialization state for managed `// materialize` assets.
-- One row per (asset, partition): the latest materialization of that slice.
-- Drives: the partition-status grid (CE observability), run-stale/gap
-- detection, and the EE backfill worklist (missing/failed partitions). The
-- `partition` column uses '' as the sentinel for an unpartitioned (whole-table)
-- materialization, since partition is part of the primary key and cannot be
-- NULL.
CREATE TYPE MATERIALIZATION_STATUS AS ENUM ('running', 'materialized', 'failed');

CREATE TABLE IF NOT EXISTS materialized_partition (
  workspace_id     VARCHAR(50) NOT NULL REFERENCES workspace(id) ON DELETE CASCADE ON UPDATE CASCADE,
  asset_kind       ASSET_KIND NOT NULL,
  asset_path       VARCHAR(255) NOT NULL,
  partition        TEXT NOT NULL DEFAULT '',
  status           MATERIALIZATION_STATUS NOT NULL,
  -- DuckLake snapshot id produced by the write; NULL while running / on
  -- failure. The pin that makes downstream reads reproducible.
  snapshot_id      BIGINT,
  row_count        BIGINT,
  job_id           UUID,
  materialized_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
  error            TEXT,
  PRIMARY KEY (workspace_id, asset_kind, asset_path, partition)
);

-- Backfill enumeration / grid "show only gaps": filter an asset's partitions
-- by status without scanning the whole table.
CREATE INDEX IF NOT EXISTS idx_materialized_partition_asset_status
  ON materialized_partition (workspace_id, asset_kind, asset_path, status);
