CREATE TABLE flow_workspace_runnables (
    flow_path VARCHAR(255) NOT NULL,
    runnable_path VARCHAR(255) NOT NULL,
    runnable_is_flow BOOLEAN NOT NULL,
    workspace_id VARCHAR(50) NOT NULL,
	FOREIGN KEY (workspace_id, flow_path) REFERENCES flow (workspace_id, path) ON DELETE CASCADE
);

CREATE INDEX flow_workspace_runnable_path_is_flow_idx ON flow_workspace_runnables (runnable_path, runnable_is_flow, workspace_id);
