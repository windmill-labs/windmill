-- Add up migration script here
-- Add up migration script here
CREATE TYPE LOG_MODE AS ENUM ('standalone', 'server', 'worker', 'agent');
CREATE TABLE log_file (
    hostname VARCHAR(255) NOT NULL,
    log_ts TIMESTAMP,
    mode LOG_MODE NOT NULL,
    file_path VARCHAR(510),
    PRIMARY KEY (hostname, mode)
);

CREATE INDEX log_file_last_updated_idx ON log_file (log_ts);