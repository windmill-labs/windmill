-- Rollback: Remove job_isolation column from worker_ping table
ALTER TABLE worker_ping DROP COLUMN IF EXISTS job_isolation;
