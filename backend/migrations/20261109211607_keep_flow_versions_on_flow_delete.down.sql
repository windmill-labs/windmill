DELETE FROM flow_version v
WHERE NOT EXISTS (SELECT 1 FROM flow f WHERE f.workspace_id = v.workspace_id AND f.path = v.path);
ALTER TABLE flow_version ADD CONSTRAINT flow_version_workspace_id_path_fkey
    FOREIGN KEY (workspace_id, path) REFERENCES flow (workspace_id, path) ON DELETE CASCADE;
-- flow_node's constraint is not restored: a node left at a deleted flow's path can be
-- shared by content with a live flow's versions, so deleting orphans could break that flow.
