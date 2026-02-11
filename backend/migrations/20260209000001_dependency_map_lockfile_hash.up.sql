-- Add column to track lockfile hash of imported scripts
-- Used to skip re-locking when imports' lockfiles haven't changed
ALTER TABLE dependency_map ADD COLUMN imported_lockfile_hash BIGINT;

-- Index for queries filtering by importer (skip-relock check, load(), clear_map_for_item)
CREATE INDEX IF NOT EXISTS dependency_map_importer_path_idx ON dependency_map (workspace_id, importer_path);
