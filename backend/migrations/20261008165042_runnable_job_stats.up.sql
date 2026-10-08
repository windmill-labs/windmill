-- Hourly resource rollup per runnable and worker group, fed by the workers.
-- Memory figures are in kB, like v2_job_completed.memory_peak.
CREATE TABLE IF NOT EXISTS runnable_job_stats (
    hour BIGINT NOT NULL,
    workspace_id VARCHAR(50) NOT NULL REFERENCES workspace(id) ON DELETE CASCADE,
    runnable_path VARCHAR(255) NOT NULL,
    worker_group TEXT NOT NULL,
    job_count INTEGER NOT NULL DEFAULT 0,
    total_duration_ms BIGINT NOT NULL DEFAULT 0,
    max_memory_peak INTEGER NOT NULL DEFAULT 0,
    sum_memory_peak BIGINT NOT NULL DEFAULT 0,
    memory_sample_count INTEGER NOT NULL DEFAULT 0,
    total_cpu_ms BIGINT NOT NULL DEFAULT 0,
    PRIMARY KEY (hour, workspace_id, runnable_path, worker_group)
);
