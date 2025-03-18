-- flow_workspace_runnables only stored runnable usages by
-- flows although apps can also use runnables

ALTER TABLE flow_workspace_runnables
RENAME TO workspace_runnable_dependencies;

ALTER TABLE workspace_runnable_dependencies ALTER flow_path DROP NOT NULL;

ALTER TABLE workspace_runnable_dependencies 
ADD COLUMN app_path VARCHAR(255);

ALTER TABLE workspace_runnable_dependencies
ADD CONSTRAINT fk_workspace_runnable_dependencies_app_path
FOREIGN KEY (app_path, workspace_id) REFERENCES app (path, workspace_id);

ALTER TABLE workspace_runnable_dependencies
ADD CONSTRAINT workspace_runnable_dependencies_path_exclusive CHECK (
  (flow_path IS NOT NULL AND app_path IS NULL) OR 
  (flow_path IS NULL AND app_path IS NOT NULL)
);

