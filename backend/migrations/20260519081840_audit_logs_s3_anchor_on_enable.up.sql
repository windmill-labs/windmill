-- When `store_audit_logs_s3` is enabled, atomically anchor the audit→object
-- store export cursor *in the enabling transaction*. The cursor lives in the
-- dedicated `background_task_state` table (NOT `global_settings`, which is
-- conceptually user-configurable instance config exposed via the settings
-- UI/export). Capturing txid_snapshot_xmin here (rather than the settings
-- row's own xmin, or a later async/first-tick read) is the only boundary that
-- is correct for the common case where audit_log() runs in a caller
-- transaction that acquired its xid before the enable: such a transaction is
-- in-flight at this snapshot, so its xid >= the snapshot xmin and its
-- post-enable audit rows are still exported.
-- The `1970-01-01` last_ts is a "bootstrap, not yet exported" sentinel: the
-- first export run uses an epoch timestamp floor (no partition pruning) so an
-- arbitrarily old / delayed backlog is not dropped, then sets a real last_ts.
-- ON CONFLICT DO NOTHING => idempotent, HA-safe, never overwrites (a
-- disable/re-enable resumes from the preserved cursor). The WHEN clause limits
-- the trigger to the `store_audit_logs_s3` row. The task name literal must
-- match `windmill_common::global_settings::AUDIT_LOGS_S3_EXPORT_TASK`.

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
                'last_ts', '1970-01-01T00:00:00+00:00'
            )
        )
        ON CONFLICT (name) DO NOTHING;
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER audit_logs_s3_anchor_trigger
AFTER INSERT OR UPDATE OF value ON global_settings
FOR EACH ROW
WHEN (NEW.name = 'store_audit_logs_s3')
EXECUTE FUNCTION audit_logs_s3_anchor_on_enable();
