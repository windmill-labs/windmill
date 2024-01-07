-- Add up migration script here

CREATE TYPE IMPORTER_KIND AS ENUM ('script', 'flow', 'app');

CREATE TABLE IF NOT EXISTS dependency_map (
    workspace_id VARCHAR(50) NOT NULL,
    importer_path VARCHAR(510) NOT NULL,
    importer_kind IMPORTER_KIND NOT NULL,
    imported_path VARCHAR(510) NOT NULL,
    PRIMARY KEY (workspace_id, importer_path, importer_kind, imported_path)
);

CREATE UNIQUE INDEX IF NOT EXISTS dependency_map_imported_path_idx ON dependency_map (workspace_id, imported_path);

