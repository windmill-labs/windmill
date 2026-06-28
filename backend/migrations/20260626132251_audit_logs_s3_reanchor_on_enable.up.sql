-- Anchors the audit→object-store export cursor when the setting is enabled.
--
-- `last_ts`/`last_oldest_inflight_ts` must be a *recent* floor, not epoch: the
-- export's `timestamp >= floor` predicate is the only partition-pruning bound (the
-- `age(xmin)` cursor is unindexable), so an epoch floor would scan the whole
-- `audit_partitioned` table on the first run and never finish under a
-- `statement_timeout`. The floor must be at or below the timestamp of any row whose
-- xid >= this snapshot xmin; the oldest in-flight `xact_start` is that bound when
-- stats are visible (no restricted role / prepared 2PC txn), else a bounded 7-day
-- window.
--
-- `ON CONFLICT DO UPDATE ... WHERE last_xmin <` keeps the cursor monotonic: a
-- re-enable re-anchors it forward (so the export resumes from ~now rather than
-- rescanning the disabled gap — that gap is the backfill's job), but it never moves
-- backwards, so it is HA-safe and can't be regressed by a slower concurrent writer.
--
-- The task name literal must match
-- `windmill_common::global_settings::AUDIT_LOGS_S3_EXPORT_TASK`.

-- Oldest in-flight `xact_start` when this role can observe *all* sessions (so the
-- min is a true cluster-wide bound), else NULL — callers substitute a conservative
-- window. The stats-visibility check goes through `is_superuser` (a preset GUC, no
-- catalog read) first, then a best-effort `pg_has_role` probe guarded by EXCEPTION:
-- `pg_has_role` reads `pg_authid`, which some managed providers (e.g. Cloud SQL)
-- forbid even to read from an elevated context, raising "Modifying pg_authid or
-- pg_auth_members is not allowed in elevated context". The EXCEPTION block runs in
-- its own subtransaction, so a failure there returns NULL (→ conservative fallback)
-- without aborting the caller — the migration-time UPDATE below, the trigger, or the
-- export task, none of which must fail just because the optimization is unavailable.
CREATE OR REPLACE FUNCTION audit_logs_s3_oldest_inflight_ts()
RETURNS timestamptz AS $$
DECLARE
    v_can_read_all_stats boolean := current_setting('is_superuser') = 'on';
BEGIN
    IF NOT v_can_read_all_stats THEN
        BEGIN
            v_can_read_all_stats := pg_has_role(current_user, 'pg_read_all_stats', 'USAGE');
        EXCEPTION WHEN OTHERS THEN
            v_can_read_all_stats := false;
        END;
    END IF;
    IF v_can_read_all_stats AND NOT EXISTS (SELECT 1 FROM pg_prepared_xacts) THEN
        RETURN (SELECT min(xact_start) FROM pg_stat_activity WHERE xact_start IS NOT NULL);
    END IF;
    RETURN NULL;
END;
$$ LANGUAGE plpgsql;

CREATE OR REPLACE FUNCTION audit_logs_s3_anchor_on_enable()
RETURNS TRIGGER AS $$
BEGIN
    IF NEW.value = to_jsonb(true)
       AND (TG_OP = 'INSERT' OR OLD.value IS DISTINCT FROM NEW.value) THEN
        INSERT INTO background_task_state (name, value)
        VALUES (
            'audit_logs_s3_export',
            jsonb_build_object(
                'last_xmin', txid_snapshot_xmin(txid_current_snapshot())::bigint,
                'last_ts', now(),
                'last_oldest_inflight_ts',
                    COALESCE(audit_logs_s3_oldest_inflight_ts(), now() - interval '7 days')
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

-- Recovery for a legacy epoch-sentinel checkpoint (`last_ts = epoch`, never
-- drained). It cannot be safely resumed: its un-drained backlog can be arbitrarily
-- old, so stamping a recent floor over the old xmin would prune the older rows while
-- the cursor advanced past them (silent loss), and keeping the epoch floor would
-- reintroduce the full scan. Re-anchor it to now like a fresh enable; the pre-anchor
-- window is recoverable via the opt-in backfill, not silently dropped.
UPDATE background_task_state
SET value = jsonb_build_object(
        'last_xmin', txid_snapshot_xmin(txid_current_snapshot())::bigint,
        'last_ts', to_jsonb(now()),
        'last_oldest_inflight_ts',
            to_jsonb(COALESCE(audit_logs_s3_oldest_inflight_ts(), now() - interval '7 days')))
WHERE name = 'audit_logs_s3_export'
  AND (value->>'last_ts')::timestamptz <= 'epoch'::timestamptz;
