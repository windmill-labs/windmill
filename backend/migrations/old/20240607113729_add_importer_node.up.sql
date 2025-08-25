-- Add up migration script here
ALTER TABLE dependency_map ADD COLUMN importer_node_id VARCHAR(255) NOT NULL DEFAULT '';
ALTER TABLE dependency_map DROP constraint dependency_map_pkey;
ALTER TABLE dependency_map ADD PRIMARY KEY (workspace_id, importer_node_id, importer_kind, importer_path, imported_path);