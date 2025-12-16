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

CREATE TABLE IF NOT EXISTS runnable_settings(
    hash               BIGINT PRIMARY KEY,
    debouncing_settings BIGINT DEFAULT NULL,
    concurrency_settings BIGINT DEFAULT NULL
);

CREATE TABLE IF NOT EXISTS job_settings(
    job_id            UUID PRIMARY KEY,
    runnable_settings BIGINT DEFAULT NULL
);

ALTER TABLE script
ADD COLUMN runnable_settings_handle BIGINT DEFAULT NULL;

ALTER TABLE v2_job_queue
ADD COLUMN runnable_settings_handle BIGINT DEFAULT NULL;


GRANT ALL ON concurrency_settings TO windmill_admin;
GRANT ALL ON concurrency_settings TO windmill_user;
GRANT ALL ON debouncing_settings TO windmill_admin;
GRANT ALL ON debouncing_settings TO windmill_user;
GRANT ALL ON runnable_settings TO windmill_admin;
GRANT ALL ON runnable_settings TO windmill_user;
GRANT ALL ON job_settings TO windmill_admin;
GRANT ALL ON job_settings TO windmill_user;
