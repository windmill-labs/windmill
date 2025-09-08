-- Add worker_group_job_stats table
CREATE TABLE IF NOT EXISTS worker_group_job_stats (
    hour BIGINT NOT NULL,
    worker_group TEXT NOT NULL,
    script_lang VARCHAR(50),
    workspace_id VARCHAR(50) NOT NULL REFERENCES workspace(id) ON DELETE CASCADE,
    job_count INTEGER NOT NULL DEFAULT 0,
    total_duration_ms BIGINT NOT NULL DEFAULT 0,
    PRIMARY KEY (hour, worker_group, script_lang, workspace_id)
);

-- Create indices for efficient querying
CREATE INDEX IF NOT EXISTS worker_group_job_stats_hour_idx ON worker_group_job_stats(hour DESC);
CREATE INDEX IF NOT EXISTS worker_group_job_stats_workspace_idx ON worker_group_job_stats(workspace_id, hour DESC);
CREATE INDEX IF NOT EXISTS worker_group_job_stats_worker_group_idx ON worker_group_job_stats(worker_group, hour DESC);