-- Add support for runtime asset detection
CREATE TYPE ASSET_DETECTION_KIND AS ENUM ('static', 'runtime');

-- Add new columns to asset table
ALTER TABLE asset
  ADD COLUMN id BIGSERIAL,
  ADD COLUMN asset_detection_kind ASSET_DETECTION_KIND NOT NULL DEFAULT 'static',
  ADD COLUMN job_id UUID REFERENCES v2_job(id) ON DELETE CASCADE;

-- Drop existing primary key
ALTER TABLE asset DROP CONSTRAINT asset_pkey;

-- Create new primary key using id
ALTER TABLE asset
  ADD PRIMARY KEY (id);

-- Create unique constraint for static assets
CREATE UNIQUE INDEX asset_static_unique_idx
  ON asset (workspace_id, path, kind, usage_path, usage_kind)
  WHERE asset_detection_kind = 'static';

-- Create unique constraint for runtime assets
CREATE UNIQUE INDEX asset_runtime_unique_idx
  ON asset (workspace_id, path, kind, usage_path, usage_kind, job_id)
  WHERE asset_detection_kind = 'runtime';

-- Add index to optimize queries filtering by detection kind
CREATE INDEX idx_asset_detection_kind ON asset (workspace_id, asset_detection_kind);

CREATE INDEX idx_asset_runtime_lookup ON asset (workspace_id, path, kind, job_id) 
  WHERE asset_detection_kind = 'runtime';

-- Add constraint: runtime assets must have job_id, static assets must not
ALTER TABLE asset
  ADD CONSTRAINT asset_job_id_check CHECK (
    (asset_detection_kind = 'static' AND job_id IS NULL) OR
    (asset_detection_kind = 'runtime' AND job_id IS NOT NULL)
  );
