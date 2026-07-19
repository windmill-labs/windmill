-- Restore the previous anchor: epoch sentinel + preserve-cursor (DO NOTHING).
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

DROP FUNCTION IF EXISTS audit_logs_s3_oldest_inflight_ts();
