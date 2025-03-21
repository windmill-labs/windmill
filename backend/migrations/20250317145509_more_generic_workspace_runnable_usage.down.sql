DROP VIEW flow_workspace_runnables;

DELETE FROM workspace_runnable_dependencies WHERE flow_path IS NULL;

ALTER TABLE workspace_runnable_dependencies
DROP CONSTRAINT flow_workspace_runnables_workspace_id_flow_path_fkey;

ALTER TABLE workspace_runnable_dependencies
ADD CONSTRAINT flow_workspace_runnables_workspace_id_flow_path_fkey
FOREIGN KEY (flow_path, workspace_id) REFERENCES flow (path, workspace_id) 
ON DELETE CASCADE;

ALTER TABLE workspace_runnable_dependencies
DROP CONSTRAINT workspace_runnable_dependencies_path_exclusive;

ALTER TABLE workspace_runnable_dependencies
DROP CONSTRAINT fk_workspace_runnable_dependencies_app_path;

ALTER TABLE workspace_runnable_dependencies DROP COLUMN app_path;

ALTER TABLE workspace_runnable_dependencies ALTER flow_path SET NOT NULL;

ALTER TABLE workspace_runnable_dependencies
RENAME TO flow_workspace_runnables;
