-- Drop indexes added in the up migration
DROP INDEX IF EXISTS idx_asset_job_pruning;
DROP INDEX IF EXISTS idx_asset_workspace_created_id;

ALTER TABLE asset
  DROP COLUMN IF EXISTS created_at,
  DROP COLUMN IF EXISTS id;

DELETE FROM asset WHERE usage_kind = 'job';