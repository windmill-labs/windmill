-- Add up migration script here
CREATE TYPE LOG_MODE AS ENUM ('standalone', 'server', 'worker', 'agent');
CREATE TABLE log_file (
    hostname VARCHAR(255) NOT NULL,
    mode LOG_MODE NOT NULL,
    file_ts TIMESTAMP NOT NULL,
    file_path VARCHAR(510) NOT NULL,
    PRIMARY KEY (hostname, mode, file_ts)
);