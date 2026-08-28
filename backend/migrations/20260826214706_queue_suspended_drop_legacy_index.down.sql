CREATE INDEX IF NOT EXISTS queue_suspended
    ON v2_job_queue (priority DESC NULLS LAST, created_at, suspend_until, suspend, tag)
    WHERE suspend_until IS NOT NULL;
