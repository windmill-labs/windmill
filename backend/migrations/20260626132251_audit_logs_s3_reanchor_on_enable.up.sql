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

-- One-time recovery for an instance upgrading while a legacy epoch-sentinel
-- checkpoint is still in place (enabled on the old code but never drained — e.g.
-- the very full-table scan this migration removes left it stuck). Such a
-- checkpoint cannot be safely resumed: its un-drained backlog can be arbitrarily
-- old, so stamping a recent floor over the old xmin would prune the older rows
-- while the cursor advanced past them (silent loss), and keeping the epoch floor
-- would reintroduce the full scan. Instead re-anchor it to now, exactly like a
-- fresh enable — the export resumes cleanly from ~now, and the pre-upgrade window
-- (which was never successfully exported) is recovered with the opt-in historical
-- backfill rather than silently dropped.
UPDATE background_task_state
SET value = jsonb_build_object(
        'last_xmin', txid_snapshot_xmin(txid_current_snapshot())::bigint,
        'last_ts', to_jsonb(now()),
        'last_oldest_inflight_ts', to_jsonb(COALESCE(
            CASE WHEN (current_setting('is_superuser') = 'on'
                        OR pg_has_role(current_user, 'pg_read_all_stats', 'USAGE'))
                      AND NOT EXISTS (SELECT 1 FROM pg_prepared_xacts)
                 THEN (SELECT min(xact_start) FROM pg_stat_activity WHERE xact_start IS NOT NULL)
                 ELSE NULL END,
            now() - interval '7 days')))
WHERE name = 'audit_logs_s3_export'
  AND (value->>'last_ts')::timestamptz <= 'epoch'::timestamptz;
