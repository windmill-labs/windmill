-- The service log ingest walked `log_file` with a cursor over `log_ts`, which is when a line
-- was written rather than when its row appeared. Rows do not arrive in that order — an upload
-- retried after a failure, a host that has just started, a batch the row limit cut mid-minute —
-- and a row that becomes visible behind the cursor is never read: it stays in `log_file` and its
-- lines stay out of search until retention drops them.
--
-- No ordering fixes this. A cursor over arrival order fails the same way, because `nextval` is
-- allocated before its INSERT commits: a row can be assigned a lower value and commit after a
-- higher one has already moved the cursor past it. Which rows are outstanding is a property of
-- the rows, so it is recorded on them.
ALTER TABLE log_file ADD COLUMN indexed_at TIMESTAMPTZ;

-- Rows that already existed are marked, not queued: on a 14-day window most were ingested long
-- ago and their raw files are gone. A sentinel rather than a timestamp, because the indexer has to
-- tell them apart from rows registered since — those start NULL — and it puts the window's worth of
-- them back on the queue on its first pass, keeping only what the columnar store can vouch for.
--
-- Not split here on the cursor the old ingest had reached. Below that cursor sits every row it
-- skipped, which is the loss this migration exists to stop; recording those as done would carry the
-- bug into its own fix.
UPDATE log_file SET indexed_at = 'epoch' WHERE indexed_at IS NULL;

-- The work queue, and the only index the ingest query needs: outstanding rows are a small
-- fraction of the table, so this stays proportional to what is left to do rather than to the
-- retention window.
CREATE INDEX index_log_file_pending ON log_file (log_ts) WHERE indexed_at IS NULL;

-- A rebuild takes rows out of the queue by the file it just read out of the store, which is
-- the one lookup that arrives without a `log_ts`: the primary key is `(hostname, log_ts)`, so
-- nothing else covers it and each batch would scan every outstanding row instead.
CREATE INDEX index_log_file_pending_path ON log_file (hostname, file_path) WHERE indexed_at IS NULL;

-- Reached once per pass while pre-migration rows survive, and never again after the first
-- conversion clears them.
CREATE INDEX index_log_file_premigration ON log_file (log_ts) WHERE indexed_at = 'epoch';
