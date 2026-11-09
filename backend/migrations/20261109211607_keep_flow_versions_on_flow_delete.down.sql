DELETE FROM flow_version v
WHERE NOT EXISTS (SELECT 1 FROM flow f WHERE f.workspace_id = v.workspace_id AND f.path = v.path);
DELETE FROM flow_node n
WHERE NOT EXISTS (SELECT 1 FROM flow f WHERE f.workspace_id = n.workspace_id AND f.path = n.path);
ALTER TABLE flow_version ADD CONSTRAINT flow_version_workspace_id_path_fkey
    FOREIGN KEY (workspace_id, path) REFERENCES flow (workspace_id, path) ON DELETE CASCADE;
ALTER TABLE flow_node ADD CONSTRAINT flow_node_path_workspace_id_fkey
    FOREIGN KEY (path, workspace_id) REFERENCES flow (path, workspace_id) ON DELETE CASCADE;
