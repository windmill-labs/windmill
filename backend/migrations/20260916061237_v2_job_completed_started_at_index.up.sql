-- Bounds the started_at window on the completed-jobs listing. Every other workspace index on
-- this table leads on completed_at, so a started_at window could only be applied as a filter
-- over all of the workspace's completed rows.
--
-- Column order mirrors ix_job_workspace_id_completed_at_all. The listing sorts on
-- v2_job.created_at, which no index on this table can supply, so what this one buys is the
-- bounded range and not the ordering.
--
-- Dropped before built: a failed concurrent build leaves an invalid index that IF NOT EXISTS
-- would accept forever, unused by the planner yet still maintained on every write.
--
-- No statement separators outside the statements below, comments included: the CONCURRENTLY
-- rewrite in windmill-api/src/db.rs splits the file on them and would run comment text as SQL.
DROP INDEX IF EXISTS ix_v2_job_completed_workspace_started_at;

CREATE INDEX ix_v2_job_completed_workspace_started_at
    ON v2_job_completed (workspace_id, started_at DESC);
