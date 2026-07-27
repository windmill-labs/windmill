-- The run page polls one job's per-relation progress every 2s, and that is the
-- only reader keyed on `job_id`: the column leads neither the primary key nor
-- the asset/status index, both shaped for the per-relation lookups that came
-- first. Small table until a partitioned asset is backfilled, then one row per
-- slice per asset.
CREATE INDEX IF NOT EXISTS idx_materialized_partition_job
    ON materialized_partition (workspace_id, job_id)
    WHERE job_id IS NOT NULL;
