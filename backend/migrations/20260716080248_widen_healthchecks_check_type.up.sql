-- Alert tags embed unbounded components (mountpoint, hostname), so they overflowed
-- varchar(50). create_alert only logs the insert error while the notification still
-- fires, so an overflowing tag re-alerts every monitor pass and never recovers.
ALTER TABLE healthchecks ALTER COLUMN check_type TYPE text;
