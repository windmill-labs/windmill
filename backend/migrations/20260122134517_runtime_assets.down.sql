-- Remove runtime asset support

-- Drop constraints and indexes
ALTER TABLE asset DROP CONSTRAINT IF EXISTS asset_job_id_check;
DROP INDEX IF EXISTS idx_asset_runtime_lookup;
DROP INDEX IF EXISTS idx_asset_detection_kind;
DROP INDEX IF EXISTS asset_runtime_unique_idx;
DROP INDEX IF EXISTS asset_static_unique_idx;

-- Drop the new primary key
ALTER TABLE asset DROP CONSTRAINT asset_pkey;

-- Restore original primary key
ALTER TABLE asset
  ADD PRIMARY KEY (workspace_id, path, kind, usage_path, usage_kind);

-- Remove new columns
ALTER TABLE asset
  DROP COLUMN IF EXISTS id,
  DROP COLUMN IF EXISTS job_id,
  DROP COLUMN IF EXISTS asset_detection_kind;

-- Drop the enum type
DROP TYPE IF EXISTS ASSET_DETECTION_KIND;
