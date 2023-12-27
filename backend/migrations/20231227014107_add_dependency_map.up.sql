-- Add up migration script here
CREATE TABLE IF NOT EXISTS dependency_map (
    importer_path VARCHAR(510) NOT NULL,
    importer_is_flow BOOLEAN NOT NULL,
    imported_path VARCHAR(510) NOT NULL,
    PRIMARY KEY (importer_path, importer_is_flow, imported_path)
);

CREATE UNIQUE INDEX IF NOT EXISTS dependency_map_imported_path_idx ON dependency_map (imported_path);

