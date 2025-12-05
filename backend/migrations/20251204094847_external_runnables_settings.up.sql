CREATE TABLE IF NOT EXISTS concurrency_settings(
    hash                        BIGINT PRIMARY KEY,
    concurrency_key             VARCHAR(255),
    concurrent_limit            INTEGER,
    concurrency_time_window_s   INTEGER
);

CREATE TABLE IF NOT EXISTS debouncing_settings(
    hash                        BIGINT PRIMARY KEY,
    debounce_key                VARCHAR(255),
    debounce_delay_s            INTEGER,
    max_total_debouncing_time   INTEGER,
    max_total_debounces_amount  INTEGER,
    debounce_args_to_accumulate TEXT[]
);

CREATE TABLE IF NOT EXISTS runnable_settings_references(
    runnable_id                 BIGINT NOT NULL,
    runnable_kind               IMPORTER_KIND NOT NULL,
    debouncing_settings_hash    BIGINT REFERENCES debouncing_settings(hash) ON DELETE RESTRICT,
    concurrency_settings_hash   BIGINT REFERENCES concurrency_settings(hash) ON DELETE RESTRICT,
    PRIMARY KEY (runnable_id, runnable_kind)
);

CREATE TABLE IF NOT EXISTS v2_jobs_settings_references(
    job_id                      UUID PRIMARY KEY, 
    debouncing_settings_hash    BIGINT REFERENCES debouncing_settings(hash) ON DELETE RESTRICT,
    concurrency_settings_hash   BIGINT REFERENCES concurrency_settings(hash) ON DELETE RESTRICT
);
