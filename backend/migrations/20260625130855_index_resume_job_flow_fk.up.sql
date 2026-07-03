-- Index resume_job's FK column to v2_job_queue. Without it, every per-job-completion
-- DELETE FROM v2_job_queue (the system's hottest delete path) cascades into a sequential
-- scan of resume_job. The table is small (only currently-suspended flows), so the scan is
-- cheap today, but the index makes the cascade an index probe and removes the footgun.
CREATE INDEX IF NOT EXISTS ix_resume_job_flow ON resume_job (flow);
