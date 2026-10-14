-- A late run finished (or, for flows, started) after the next occurrence was due, so that
-- occurrence was missed. late_run_streak counts late runs in a row and is reset by a run on
-- time; missed_occurrences is what the latest streak missed and last_missed_at when, both
-- kept after the streak ends so a recent one stays visible. An edit or toggle clears all.
ALTER TABLE schedule ADD COLUMN late_run_streak INTEGER NOT NULL DEFAULT 0;
ALTER TABLE schedule ADD COLUMN missed_occurrences INTEGER NOT NULL DEFAULT 0;
ALTER TABLE schedule ADD COLUMN last_missed_at TIMESTAMPTZ;
