-- Set on a version deployed with `apply_to_perpetual_runs`: a perpetual run of an older version at
-- the same path starts its next run on this version instead of its own.
ALTER TABLE script ADD COLUMN IF NOT EXISTS apply_to_perpetual_runs BOOLEAN;
