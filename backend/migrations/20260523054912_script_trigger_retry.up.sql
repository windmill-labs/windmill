-- Cascade retry policy (`// retry <count> [<delay>]`). Stored per
-- `script_trigger` row — like `join_all`, it is a script-level property
-- (every row for a given runnable carries the same value) but lives
-- alongside the edge so the dispatcher reads everything it needs from a
-- single query. NULL = no retry (current behaviour); set = re-run on
-- failure up to `retry_count` times, waiting `retry_delay_s` seconds
-- between attempts (0 = back-to-back).
ALTER TABLE script_trigger
  ADD COLUMN retry_count   SMALLINT,
  ADD COLUMN retry_delay_s INTEGER;
