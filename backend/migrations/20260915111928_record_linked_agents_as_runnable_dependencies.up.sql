-- A flow step linked to a saved agent (an `ai_agent` resource) is recorded next to the scripts and
-- subflows the flow runs, so renaming the agent can name the flows it would break. An agent row is
-- neither a script nor a flow: readers of script usages have to exclude it.
ALTER TABLE workspace_runnable_dependencies
    ADD COLUMN runnable_is_agent BOOLEAN NOT NULL DEFAULT false;

-- A script step and a linked agent can share a path. Without the flag in the key, the second
-- insert's ON CONFLICT DO NOTHING would silently drop one of the two rows.
DROP INDEX flow_workspace_without_hash_unique_idx;

CREATE UNIQUE INDEX flow_workspace_without_hash_unique_idx
    ON workspace_runnable_dependencies (flow_path, runnable_path, runnable_is_flow, runnable_is_agent, workspace_id)
    WHERE script_hash IS NULL;

-- The worker only records a flow when it is next deployed, so seed the ones already linking an
-- agent from their current value.
INSERT INTO workspace_runnable_dependencies (flow_path, runnable_path, runnable_is_flow, runnable_is_agent, workspace_id)
SELECT DISTINCT f.path, agent_ref #>> '{}', false, true, f.workspace_id
FROM flow f
CROSS JOIN LATERAL jsonb_path_query(f.value, 'lax $.** ? (@.type == "aiagent" && @.agent.type() == "string").agent') AS agent_ref
ON CONFLICT DO NOTHING;
