-- flow_workspace_runnables only stored runnable usages by
-- flows although apps can also use runnables

CREATE TABLE workspace_runnable_dependencies (
    flow_path VARCHAR(255) NULL,
    app_path VARCHAR(255) NULL,
    runnable_path VARCHAR(255) NOT NULL,
    script_hash BIGINT NULL,
    runnable_is_flow BOOLEAN NOT NULL,
    workspace_id VARCHAR(50) NOT NULL,
	FOREIGN KEY (workspace_id, flow_path) REFERENCES flow (workspace_id, path) ON DELETE CASCADE ON UPDATE CASCADE,
	FOREIGN KEY (workspace_id, app_path) REFERENCES app (workspace_id, path) ON DELETE CASCADE ON UPDATE CASCADE
);

ALTER TABLE workspace_runnable_dependencies
ADD CONSTRAINT workspace_runnable_dependencies_path_exclusive CHECK (
  (flow_path IS NOT NULL AND app_path IS NULL) OR 
  (flow_path IS NULL AND app_path IS NOT NULL)
);

DROP INDEX IF EXISTS workspace_runnable_dependencies_runnable_path_is_flow_idx;
CREATE INDEX workspace_runnable_dependencies_runnable_path_is_flow_idx ON workspace_runnable_dependencies (runnable_path, runnable_is_flow, workspace_id);

-- Prevent duplicate dependencies from same flow
DROP INDEX IF EXISTS workspace_runnable_dependencies_without_hash_unique_idx;
DROP INDEX IF EXISTS workspace_runnable_dependencies_with_hash_unique_idx;
CREATE UNIQUE INDEX workspace_runnable_dependencies_without_hash_unique_idx ON workspace_runnable_dependencies (flow_path, runnable_path, runnable_is_flow, workspace_id) WHERE script_hash IS NULL;
CREATE UNIQUE INDEX workspace_runnable_dependencies_with_hash_unique_idx ON workspace_runnable_dependencies (flow_path, runnable_path, script_hash, runnable_is_flow, workspace_id) WHERE script_hash IS NOT NULL;

-- Prevent duplicate dependencies from same app
DROP INDEX IF EXISTS workspace_runnable_dependencies_without_hash_unique_idx;
DROP INDEX IF EXISTS workspace_runnable_dependencies_with_hash_unique_idx;
CREATE UNIQUE INDEX workspace_runnable_dependencies_without_hash_unique_idx ON workspace_runnable_dependencies (app_path, runnable_path, runnable_is_flow, workspace_id) WHERE script_hash IS NULL;
CREATE UNIQUE INDEX workspace_runnable_dependencies_with_hash_unique_idx ON workspace_runnable_dependencies (app_path, runnable_path, script_hash, runnable_is_flow, workspace_id) WHERE script_hash IS NOT NULL;