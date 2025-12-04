CREATE TYPE RUNNABLE_SETTINGS_REFERENCER_KIND AS ENUM ('script', 'flow', 'app', 'job');

CREATE TABLE IF NOT EXISTS runnable_concurrency_settings(
    hash                        BIGINT PRIMARY KEY,
    concurrency_key             VARCHAR(255),
    concurrent_limit            INTEGER,
    concurrency_time_window_s   INTEGER
);

CREATE TABLE IF NOT EXISTS runnable_debouncing_settings(
    hash                        BIGINT PRIMARY KEY,
    debounce_key                VARCHAR(255),
    debounce_delay_s            INTEGER,
    max_total_debouncing_time   INTEGER,
    max_total_debounces_amount  INTEGER,
    debounce_args_to_accumulate TEXT[]
);

CREATE TABLE IF NOT EXISTS runnable_settings_references(
    referencer_id               BIGINT PRIMARY KEY,
    referencer_kind             RUNNABLE_SETTINGS_REFERENCER_KIND NOT NULL,
    debouncing_settings_hash    BIGINT REFERENCES runnable_debouncing_settings(hash) ON DELETE RESTRICT,
    concurrency_settings_hash   BIGINT REFERENCES runnable_debouncing_settings(hash) ON DELETE RESTRICT
);
