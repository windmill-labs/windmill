-- Set when the latest chained occurrence was pushed past at least one due slot of the
-- cron (the previous run finished, or started, too late); cleared by the next clean push.
ALTER TABLE schedule ADD COLUMN skipped_occurrences INTEGER;
ALTER TABLE schedule ADD COLUMN skipped_at TIMESTAMPTZ;
