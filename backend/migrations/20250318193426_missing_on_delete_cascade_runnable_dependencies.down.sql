ALTER TABLE workspace_runnable_dependencies
DROP CONSTRAINT fk_workspace_runnable_dependencies_app_path;

ALTER TABLE workspace_runnable_dependencies
ADD CONSTRAINT fk_workspace_runnable_dependencies_app_path
FOREIGN KEY (app_path, workspace_id) REFERENCES app (path, workspace_id);
