CREATE TABLE flow_workspace_runnables (
    flow_path VARCHAR(255) NOT NULL,
    runnable_path VARCHAR(255) NOT NULL,
    script_hash BIGINT NULL,
    runnable_is_flow BOOLEAN NOT NULL,
    workspace_id VARCHAR(50) NOT NULL,
	FOREIGN KEY (workspace_id, flow_path) REFERENCES flow (workspace_id, path) ON DELETE CASCADE
);

CREATE UNIQUE INDEX flow_workspace_without_hash_unique_idx ON flow_workspace_runnables (flow_path, runnable_path, runnable_is_flow, workspace_id) WHERE script_hash IS NULL;
CREATE UNIQUE INDEX flow_workspace_with_hash_unique_idx ON flow_workspace_runnables (flow_path, runnable_path, script_hash, runnable_is_flow, workspace_id) WHERE script_hash IS NOT NULL;
CREATE INDEX flow_workspace_runnable_path_is_flow_idx ON flow_workspace_runnables (runnable_path, runnable_is_flow, workspace_id);
