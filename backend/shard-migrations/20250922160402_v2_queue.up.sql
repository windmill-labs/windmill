-- Add up migration script here
CREATE TABLE IF NOT EXISTS v2_job_queue (
    id UUID PRIMARY KEY REFERENCES v2_job(id) ON DELETE CASCADE,
    workspace_id VARCHAR(100) NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    started_at TIMESTAMP WITH TIME ZONE,
    scheduled_for TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    running BOOLEAN NOT NULL DEFAULT FALSE,
    canceled_by VARCHAR(100),
    canceled_reason TEXT,
    suspend INTEGER,
    suspend_until TIMESTAMP WITH TIME ZONE,
    tag VARCHAR(255),
    priority SMALLINT,
    worker VARCHAR(100),
    extras JSONB
);

-- Create indices for v2_job_queue
CREATE INDEX IF NOT EXISTS queue_sort_v2 ON v2_job_queue (priority DESC NULLS LAST, scheduled_for, tag) 
    WHERE (running = false);
CREATE INDEX IF NOT EXISTS queue_suspended ON v2_job_queue (priority DESC NULLS LAST, created_at, suspend_until, suspend, tag) 
    WHERE (suspend_until IS NOT NULL);
CREATE INDEX IF NOT EXISTS root_queue_index_by_path ON v2_job_queue (workspace_id, created_at);
CREATE INDEX IF NOT EXISTS v2_job_queue_suspend ON v2_job_queue (workspace_id, suspend) WHERE (suspend > 0);