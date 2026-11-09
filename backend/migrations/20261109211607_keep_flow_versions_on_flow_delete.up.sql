-- A deleted flow's versions and nodes outlive its row so that runs in flight can finish;
-- trash removal purges the versions (trashbin::remove_trash_sql), workspace deletion both.
ALTER TABLE flow_version DROP CONSTRAINT IF EXISTS flow_version_workspace_id_path_fkey;
ALTER TABLE flow_node DROP CONSTRAINT IF EXISTS flow_node_path_workspace_id_fkey;
