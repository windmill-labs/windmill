-- Add support for runtime asset detection
CREATE TYPE ASSET_DETECTION_KIND AS ENUM ('static', 'runtime');

-- Add new columns to asset table
ALTER TABLE asset
  ADD COLUMN asset_detection_kind ASSET_DETECTION_KIND NOT NULL DEFAULT 'static',
  ADD COLUMN job_id UUID REFERENCES v2_job(id) ON DELETE CASCADE;

-- Drop existing primary key
ALTER TABLE asset DROP CONSTRAINT asset_pkey;

-- Create new primary key that includes asset_detection_kind
ALTER TABLE asset
  ADD PRIMARY KEY (workspace_id, path, kind, usage_path, usage_kind, asset_detection_kind);

-- Add index to optimize queries filtering by detection kind
CREATE INDEX idx_asset_detection_kind ON asset (workspace_id, asset_detection_kind);

-- Add index to optimize queries by job_id for runtime assets
CREATE INDEX idx_asset_job_id ON asset (job_id) WHERE job_id IS NOT NULL;

-- Add constraint: runtime assets must have job_id, static assets must not
ALTER TABLE asset
  ADD CONSTRAINT asset_job_id_check CHECK (
    (asset_detection_kind = 'static' AND job_id IS NULL) OR
    (asset_detection_kind = 'runtime' AND job_id IS NOT NULL)
  );
