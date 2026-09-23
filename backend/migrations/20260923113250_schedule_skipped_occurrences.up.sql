-- The current streak of chained runs that each skipped at least one due slot of the cron
-- (the previous run finished, or started, too late), reset by a clean run or an edit.
-- skipped_at is the latest skip and outlives the streak, so a one-off stays visible.
ALTER TABLE schedule ADD COLUMN skipped_runs INTEGER NOT NULL DEFAULT 0;
ALTER TABLE schedule ADD COLUMN skipped_occurrences INTEGER NOT NULL DEFAULT 0;
ALTER TABLE schedule ADD COLUMN skipped_at TIMESTAMPTZ;
