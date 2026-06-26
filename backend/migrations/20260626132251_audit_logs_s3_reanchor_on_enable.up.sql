-- Re-anchor the audit→object-store export cursor on every enable (including a
-- re-enable after a disable), and anchor a *recent* timestamp floor instead of
-- the epoch sentinel.
--
-- The previous version wrote `last_ts = epoch` and used `ON CONFLICT DO NOTHING`
-- (preserve the old cursor). Two consequences hurt large instances:
--   * the epoch floor disables partition pruning, so the first export run
--     sequentially scans the entire `audit_partitioned` table — under a
--     `statement_timeout` (e.g. Aiven) it never completes, so the cursor never
--     advances and nothing is ever exported;
--   * preserving the cursor across a long disable means re-enabling tries to
--     backfill the whole disabled window by the same unindexable `age(xmin)`
--     scan, with the same outcome.
--
-- Now the anchor records a recent floor (the oldest in-flight `xact_start` when
-- the stats privilege is available — a sound lower bound for any row whose xid
-- >= this snapshot xmin — else a bounded 7-day window), and `ON CONFLICT DO
-- UPDATE` advances the cursor to the current snapshot xmin on re-enable (never
-- backwards, so it stays HA-safe). Exporting the gap a disable left behind is
-- the job of the opt-in historical backfill, not this steady-state cursor.
--
-- The task name literal must match
-- `windmill_common::global_settings::AUDIT_LOGS_S3_EXPORT_TASK`.

CREATE OR REPLACE FUNCTION audit_logs_s3_anchor_on_enable()
RETURNS TRIGGER AS $$
DECLARE
    v_floor timestamptz;
BEGIN
    IF NEW.value = to_jsonb(true)
       AND (TG_OP = 'INSERT' OR OLD.value IS DISTINCT FROM NEW.value) THEN
        v_floor := COALESCE(
            CASE WHEN (current_setting('is_superuser') = 'on'
                        OR pg_has_role(current_user, 'pg_read_all_stats', 'USAGE'))
                      AND NOT EXISTS (SELECT 1 FROM pg_prepared_xacts)
                 THEN (SELECT min(xact_start) FROM pg_stat_activity WHERE xact_start IS NOT NULL)
                 ELSE NULL END,
            now() - interval '7 days');
        INSERT INTO background_task_state (name, value)
        VALUES (
            'audit_logs_s3_export',
            jsonb_build_object(
                'last_xmin', txid_snapshot_xmin(txid_current_snapshot())::bigint,
                'last_ts', now(),
                'last_oldest_inflight_ts', v_floor
            )
        )
        ON CONFLICT (name) DO UPDATE
            SET value = EXCLUDED.value
            WHERE (background_task_state.value->>'last_xmin')::bigint
                  < (EXCLUDED.value->>'last_xmin')::bigint;
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- One-time fixup for an instance upgrading while a legacy epoch sentinel cursor
-- is still in place (enabled on the old version but never drained, e.g. object
-- store down): stamp a recent floor so the next run prunes to recent partitions
-- instead of scanning all history. The xid cursor is left untouched (the drain
-- continues); only rows older than the new floor — possible solely via a
-- transaction open longer than the 7-day window — fall outside it.
UPDATE background_task_state
SET value = value || jsonb_build_object(
        'last_ts', to_jsonb(now()),
        'last_oldest_inflight_ts', to_jsonb(now() - interval '7 days'))
WHERE name = 'audit_logs_s3_export'
  AND (value->>'last_ts')::timestamptz <= 'epoch'::timestamptz;
