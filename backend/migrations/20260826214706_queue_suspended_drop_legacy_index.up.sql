-- Retires the index queue_suspended_v2 replaces. Separate from the migration that builds it
-- so that one is only ever replayed while this index still exists: sqlx records a migration
-- only after all its statements run, so a process that dies before the record is written
-- replays the build, and its leading DROP would otherwise be destroying the sole usable
-- index rather than an interrupted build.
DROP INDEX IF EXISTS queue_suspended;
