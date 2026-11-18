-- Per-decision log of what the asset-trigger dispatcher did after each
-- producer job completed. One row per (producer, subscriber, asset write)
-- decision: dispatched, debounced, join_pending (partial AND-join), or
-- skipped (with reason). Surfaced on the producer's job detail page so
-- the cascade is no longer invisible when the producer "succeeds" but
-- no child appears.
--
-- Retention: the FK to v2_job(id) ON DELETE CASCADE means the existing
-- retention sweep (monitor.rs delete_expired_jobs_batch -> DELETE FROM
-- v2_job WHERE id = ANY(...)) reaps these rows along with their producer.
-- No separate cleanup path needed.
--
-- Idempotency guards (duplicate_object / IF NOT EXISTS) are load-bearing:
-- re-applying this migration after a squash must be a no-op.
DO $$ BEGIN
  CREATE TYPE DISPATCH_OUTCOME AS ENUM (
    'dispatched',
    'join_pending',
    'skipped'
  );
EXCEPTION WHEN duplicate_object THEN NULL;
END $$;

CREATE TABLE IF NOT EXISTS dispatch_event (
  id              BIGSERIAL PRIMARY KEY,
  workspace_id    VARCHAR(50) NOT NULL REFERENCES workspace(id) ON DELETE CASCADE ON UPDATE CASCADE,
  producer_job_id UUID NOT NULL REFERENCES v2_job(id) ON DELETE CASCADE,
  subscriber_path VARCHAR(255) NOT NULL,
  asset_kind      ASSET_KIND NOT NULL,
  asset_path      TEXT NOT NULL,
  outcome         DISPATCH_OUTCOME NOT NULL,
  -- Set for 'dispatched'. Intentionally not FK'd: the subscriber job may
  -- be retention-reaped independently, and we still want the row to
  -- record "we dispatched <id>" historically (UI renders a dead link).
  child_job_id    UUID,
  partition       TEXT,
  -- AND-join progress at decision time. NULL for non-join subscribers.
  received_inputs INTEGER,
  required_inputs INTEGER,
  -- Effective debounce window applied to this dispatch (NULL = none).
  debounce_s      INTEGER,
  -- Free-text discriminator for 'skipped' outcomes (self_loop,
  -- case3_non_partition_bearing, case3_missing_partition, cycle_detected, ...).
  reason          TEXT,
  created_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Primary access pattern: list events for one producer (the job detail
-- panel). Ordered scans by id give chronological order for free.
CREATE INDEX IF NOT EXISTS idx_dispatch_event_producer
  ON dispatch_event (producer_job_id, id);

-- Backs the asset-graph edge listing (jobs.rs list_asset_dispatch_edges):
--   WHERE workspace_id = $1 AND subscriber_path LIKE 'prefix%'
--         AND created_at >= $3
--   ORDER BY created_at DESC, id DESC
-- The (producer_job_id, id) index above doesn't help this access path, so
-- without this one a high-volume dispatch_event seq-scans + sorts.
-- text_pattern_ops makes the anchored LIKE prefix (built as `path_start || '%'`)
-- index-usable regardless of the column collation; created_at DESC matches
-- the ORDER BY so Postgres can satisfy ordering from the index.
CREATE INDEX IF NOT EXISTS idx_dispatch_event_subscriber
  ON dispatch_event (workspace_id, subscriber_path text_pattern_ops, created_at DESC);
