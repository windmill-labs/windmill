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
