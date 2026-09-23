DELETE FROM workspace_runnable_dependencies WHERE runnable_is_agent;

DROP INDEX flow_workspace_without_hash_unique_idx;

CREATE UNIQUE INDEX flow_workspace_without_hash_unique_idx
    ON workspace_runnable_dependencies (flow_path, runnable_path, runnable_is_flow, workspace_id)
    WHERE script_hash IS NULL;

ALTER TABLE workspace_runnable_dependencies DROP COLUMN runnable_is_agent;
