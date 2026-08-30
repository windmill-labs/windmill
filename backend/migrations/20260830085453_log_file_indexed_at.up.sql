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

-- Rows that already existed are the ingest's own history. Marked with a sentinel rather than
-- `now()` so the first pass can tell them from rows it has ingested itself: it knows the
-- watermark the old cursor had reached, and returns the ones above it to the queue. Doing that
-- here instead would need the watermark, which lives in object storage.
UPDATE log_file SET indexed_at = 'epoch' WHERE indexed_at IS NULL;

-- The work queue, and the only index the ingest query needs: outstanding rows are a small
-- fraction of the table, so this stays proportional to what is left to do rather than to the
-- retention window.
CREATE INDEX index_log_file_pending ON log_file (log_ts) WHERE indexed_at IS NULL;

-- Reached once per pass while pre-migration rows survive, and never again after they age out.
CREATE INDEX index_log_file_premigration ON log_file (log_ts) WHERE indexed_at = 'epoch';

-- Rows a pass has published to the store but not yet confirmed. Normally empty — it holds a
-- batch only while a pass is inside its checkpoint — but the pass that follows an interrupted
-- one reads it before deciding anything else.
CREATE INDEX index_log_file_claimed ON log_file (log_ts) WHERE indexed_at = '-infinity';
