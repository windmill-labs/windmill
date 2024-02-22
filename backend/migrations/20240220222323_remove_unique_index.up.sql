-- Add up migration script here
DROP INDEX dependency_map_imported_path_idx;

CREATE INDEX IF NOT EXISTS dependency_map_imported_path_idx ON dependency_map (workspace_id, imported_path);
