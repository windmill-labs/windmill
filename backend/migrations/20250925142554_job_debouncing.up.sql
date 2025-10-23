-- Debounce key table: Maps debounce keys to job IDs to enable job consolidation
-- When multiple jobs with the same debounce key are scheduled within a delay window,
-- they get merged into a single job execution to avoid redundant work
-- key: Unique identifier like "workspace_id:path:dependency" or custom template
-- job_id: The UUID of the job that will handle all debounced requests
CREATE TABLE debounce_key (
    key VARCHAR(255) NOT NULL,
    job_id uuid NOT NULL,
    PRIMARY KEY (key)
);

-- Debounce stale data table: Accumulates nodes/components that need relocking across debounced jobs
-- When dependency jobs are debounced, this table collects all the to_relock data
-- so the final consolidated job knows which nodes/components to process
-- job_id: The job that will process the accumulated data
-- to_relock: Array of node/component identifiers that need dependency updates
CREATE TABLE debounce_stale_data (
    job_id uuid NOT NULL,
    to_relock TEXT[],
    PRIMARY KEY (job_id)
);

-- TODO: Prune on move/deletion
-- But normally this will persist across runs.
-- CREATE TABLE unlocked_script_latest_version (
--     key VARCHAR(255) NOT NULL,
--     version BIGINT NOT NULL,
--     PRIMARY KEY (key)
-- );
