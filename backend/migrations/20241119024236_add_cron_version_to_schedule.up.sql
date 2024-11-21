-- Add existing rows to have 'v1' as the cron_version
ALTER TABLE schedule ADD COLUMN cron_version TEXT DEFAULT 'v1';