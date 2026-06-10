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
CREATE TYPE DISPATCH_OUTCOME AS ENUM (
  'dispatched',
  'join_pending',
  'skipped'
);

CREATE TABLE dispatch_event (
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
  -- case3_non_partition_bearing, case3_missing_partition, depth_cap, ...).
  reason          TEXT,
  created_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Primary access pattern: list events for one producer (the job detail
-- panel). Ordered scans by id give chronological order for free.
CREATE INDEX idx_dispatch_event_producer
  ON dispatch_event (producer_job_id, id);
