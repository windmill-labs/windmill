DROP INDEX IF EXISTS idx_schedule_managed_by_runnable_path;
ALTER TABLE schedule DROP COLUMN IF EXISTS managed_by_runnable_path;
