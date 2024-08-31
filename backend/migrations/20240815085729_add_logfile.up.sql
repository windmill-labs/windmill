-- Add up migration script here
-- Add up migration script here
CREATE TYPE LOG_MODE AS ENUM ('standalone', 'server', 'worker', 'agent', 'indexer');
CREATE TABLE log_file (
    hostname VARCHAR(255) NOT NULL,
    log_ts TIMESTAMP,
    ok_lines BIGINT,
    err_lines BIGINT,
    mode LOG_MODE NOT NULL,
    worker_group VARCHAR(255),
    file_path VARCHAR(510) NOT NULL,
    PRIMARY KEY (hostname, log_ts)
);

CREATE INDEX log_file_log_ts_idx ON log_file (log_ts);
CREATE INDEX log_file_hostname_log_ts_idx ON log_file (hostname, log_ts);