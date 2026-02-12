-- Add column to track lockfile hash of imported scripts
-- Used to skip re-locking when imports' lockfiles haven't changed
ALTER TABLE dependency_map ADD COLUMN imported_lockfile_hash BIGINT;

-- Index for queries filtering by importer (skip-relock check, load(), clear_map_for_item)
CREATE INDEX IF NOT EXISTS dependency_map_importer_path_idx ON dependency_map (workspace_id, importer_path);

-- Stores lockfile/content hashes to detect when imports' locks have changed
CREATE TABLE lock_hash (
    workspace_id VARCHAR(50) NOT NULL REFERENCES workspace(id) ON DELETE CASCADE,
    path VARCHAR(255) NOT NULL,
    lockfile_hash BIGINT NOT NULL,
    PRIMARY KEY (workspace_id, path)
);
