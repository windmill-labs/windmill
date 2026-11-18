-- Postgres has no ALTER TYPE ... DROP VALUE for enums. The 'freshness' value
-- stays even on rollback, consistent with prior job_trigger_kind additions
-- (see 20260510174213_asset_trigger_dispatch).
DROP INDEX IF EXISTS idx_script_pipeline_freshness_scan;
DROP TABLE IF EXISTS pipeline_freshness_state;
