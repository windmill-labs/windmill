-- flow_workspace_runnables only stored runnable usages by
-- flows although apps can also use runnables

ALTER TABLE flow_workspace_runnables
RENAME TO workspace_runnable_dependencies;

ALTER TABLE workspace_runnable_dependencies ALTER flow_path DROP NOT NULL;

ALTER TABLE workspace_runnable_dependencies 
ADD COLUMN app_path VARCHAR(255);

ALTER TABLE workspace_runnable_dependencies
ADD CONSTRAINT workspace_runnable_dependencies_path_exclusive CHECK (
  (flow_path IS NOT NULL AND app_path IS NULL) OR 
  (flow_path IS NULL AND app_path IS NOT NULL)
);

ALTER TABLE workspace_runnable_dependencies
ADD CONSTRAINT fk_workspace_runnable_dependencies_app_path
FOREIGN KEY (app_path, workspace_id) REFERENCES app (path, workspace_id) 
ON DELETE CASCADE 
ON UPDATE CASCADE;


ALTER TABLE workspace_runnable_dependencies
DROP CONSTRAINT flow_workspace_runnables_workspace_id_flow_path_fkey;

ALTER TABLE workspace_runnable_dependencies
ADD CONSTRAINT flow_workspace_runnables_workspace_id_flow_path_fkey
FOREIGN KEY (flow_path, workspace_id) REFERENCES flow (path, workspace_id) 
ON DELETE CASCADE 
ON UPDATE CASCADE;


CREATE UNIQUE INDEX app_workspace_without_hash_unique_idx ON workspace_runnable_dependencies (app_path, runnable_path, runnable_is_flow, workspace_id) WHERE script_hash IS NULL;
CREATE UNIQUE INDEX app_workspace_with_hash_unique_idx ON workspace_runnable_dependencies (app_path, runnable_path, script_hash, runnable_is_flow, workspace_id) WHERE script_hash IS NOT NULL;


-- This is to maintain compatibility with old workers
CREATE VIEW flow_workspace_runnables AS 
SELECT flow_path, runnable_path, script_hash, runnable_is_flow, workspace_id
FROM workspace_runnable_dependencies;