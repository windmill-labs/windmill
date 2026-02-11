-- Stores lockfile/content hashes to detect when imports' locks have changed
CREATE TABLE lock_hash (
    workspace_id VARCHAR(50) NOT NULL,
    path VARCHAR(255) NOT NULL,
    lockfile_hash BIGINT NOT NULL,
    PRIMARY KEY (workspace_id, path)
);
