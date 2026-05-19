-- AND-join barrier (`// trigger all`). When true, the subscriber runs only
-- once every partition-bearing input has materialized at the same partition
-- (plus every reference input exists), rather than firing on any input (OR,
-- the default). Stored per script_trigger row — it is a script-level
-- property so every row for a given (workspace, runnable) carries the same
-- value; this matches the wipe-and-reinsert-on-deploy pattern and keeps the
-- subscriber lookup a single query.
ALTER TABLE script_trigger
  ADD COLUMN join_all BOOLEAN NOT NULL DEFAULT FALSE;
