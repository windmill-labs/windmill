DROP INDEX IF EXISTS idx_schedule_managed;
ALTER TABLE schedule DROP COLUMN IF EXISTS managed;
