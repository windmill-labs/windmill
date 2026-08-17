-- Add indexes on the time column alone to speed up the periodic cleanup DELETEs in monitor.rs:
--   DELETE FROM concurrency_key WHERE ended_at <= now() - ($1::bigint::text || ' s')::interval
--   DELETE FROM autoscaling_event WHERE applied_at <= now() - ($1::bigint::text || ' s')::interval
-- The existing composite indexes (key, ended_at DESC) and (worker_group, applied_at) cannot be
-- used efficiently when filtering only on the time column, so Postgres falls back to a seq scan.
CREATE INDEX IF NOT EXISTS concurrency_key_ended_at_only_idx ON concurrency_key (ended_at);
CREATE INDEX IF NOT EXISTS autoscaling_event_applied_at_idx ON autoscaling_event (applied_at);
