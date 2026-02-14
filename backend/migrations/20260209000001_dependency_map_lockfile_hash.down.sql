DROP TABLE IF EXISTS lock_hash;
DROP INDEX IF EXISTS dependency_map_importer_path_idx;
ALTER TABLE dependency_map DROP COLUMN IF EXISTS imported_lockfile_hash;
