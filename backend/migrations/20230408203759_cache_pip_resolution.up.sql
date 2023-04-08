-- Add up migration script here
CREATE TABLE pip_resolution_cache(
    hash VARCHAR(255) PRIMARY KEY,
    expiration TIMESTAMP NOT NULL,
    lockfile TEXT NOT NULL
);