-- Track schedules auto-created from a materializer's `// schedule "<cron>"`
-- annotation so reconciliation can update / drop them on subsequent deploys
-- without touching schedules a user created manually. Null for all
-- pre-existing rows.
ALTER TABLE schedule ADD COLUMN IF NOT EXISTS managed_by_runnable_path VARCHAR(255) DEFAULT NULL;

-- One managed schedule per (workspace, runnable). Index supports the
-- reconciliation lookup ("does this script already have a managed schedule?")
-- and the cleanup-on-delete query.
CREATE INDEX IF NOT EXISTS idx_schedule_managed_by_runnable_path
    ON schedule (workspace_id, managed_by_runnable_path)
    WHERE managed_by_runnable_path IS NOT NULL;
