-- The run page polls a job's per-relation progress every 2s, and the cascade
-- reads the same shape once per dbt completion. Both filter on `job_id`, which
-- leads neither the primary key nor the asset/status index -- those are shaped
-- for the per-relation lookups that came first.
CREATE INDEX IF NOT EXISTS idx_materialized_partition_job
    ON materialized_partition (workspace_id, job_id)
    WHERE job_id IS NOT NULL;
