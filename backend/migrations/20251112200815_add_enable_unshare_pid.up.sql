-- Add job_isolation column to worker_ping table
-- This tracks which job isolation method the worker is using: 'nsjail', 'unshare', or 'none'
-- Nullable for backwards compatibility - old workers will report NULL
ALTER TABLE worker_ping ADD COLUMN job_isolation TEXT;
