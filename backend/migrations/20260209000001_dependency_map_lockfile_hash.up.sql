-- Add column to track lockfile hash of imported scripts
-- Used to skip re-locking when imports' lockfiles haven't changed
ALTER TABLE dependency_map ADD COLUMN imported_lockfile_hash BIGINT;
