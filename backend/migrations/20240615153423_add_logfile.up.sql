-- Add up migration script here
CREATE TYPE LOG_MODE AS ENUM ('standalone', 'server', 'worker', 'agent');
CREATE TABLE log_file (
    hostname VARCHAR(255) NOT NULL,
    mode LOG_MODE NOT NULL,
    file_tss TIMESTAMP[],
    file_paths VARCHAR(510)[],
    last_updated TIMESTAMP,
    PRIMARY KEY (hostname, mode)
);

CREATE INDEX log_file_last_updated_idx ON log_file (last_updated);