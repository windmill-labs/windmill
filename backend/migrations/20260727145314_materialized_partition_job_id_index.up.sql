-- `job_id` leads neither the primary key nor the asset/status index, both shaped
-- for the per-relation lookups that came first — but the closing sweep a dbt run
-- makes (`terminalize_running_relations`) reads and writes by it, settling the
-- models a cancelled or timed-out run left `running`.
-- Small table until a partitioned asset is backfilled, then one row per slice
-- per asset.
CREATE INDEX IF NOT EXISTS idx_materialized_partition_job
    ON materialized_partition (workspace_id, job_id)
    WHERE job_id IS NOT NULL;
