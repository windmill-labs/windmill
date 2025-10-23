CREATE TABLE debounce_key (
    key VARCHAR(255) NOT NULL,
    job_id uuid NOT NULL,
    PRIMARY KEY (key)
);

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
