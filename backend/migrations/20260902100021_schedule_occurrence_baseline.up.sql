-- Occurrences before this point are not paired when counting skipped ones: a gap
-- spanning a pause, a cron change, a re-enable or a reconciler re-arm is not a
-- schedule that lost runs. NULL on existing rows means "no boundary known", which
-- reads the whole history rather than blinding a daily schedule for 20 days.
ALTER TABLE schedule ADD COLUMN occurrence_baseline_at TIMESTAMPTZ;
