-- A deleted flow's versions and nodes outlive its row so that runs in flight can finish:
-- its versions are set aside (trashbin::set_aside_flow_versions) and purged with its trash
-- entry, its nodes stay until the workspace is deleted.
ALTER TABLE flow_version DROP CONSTRAINT IF EXISTS flow_version_workspace_id_path_fkey;
ALTER TABLE flow_node DROP CONSTRAINT IF EXISTS flow_node_path_workspace_id_fkey;
