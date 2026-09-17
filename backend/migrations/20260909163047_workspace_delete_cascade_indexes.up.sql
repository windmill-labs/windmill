-- The FK columns that cascade when a workspace's apps and flows are deleted. Unindexed,
-- Postgres seq-scans the whole child table once per deleted parent row, making a workspace
-- delete cost O(apps and flows deleted x rows in the instance). Fork deletion is where that
-- bites: a fork clones its parent's apps, flows and entire app version history.
--
-- The workspace_runnable_dependencies pair are partial because the table's check constraint
-- makes app_path and flow_path mutually exclusive, halving each index -- the cascade's
-- equality on the path proves the predicate. The table's existing path indexes are partial on
-- script_hash, which the cascade does not constrain, so they cannot serve it.
--
-- Dropped before built: a failed concurrent build leaves an invalid index that IF NOT EXISTS
-- would accept forever, unused by the planner yet still maintained on every write.
--
-- No statement separators outside the statements below, comments included: the CONCURRENTLY
-- rewrite in windmill-api/src/db.rs splits the file on them and would run comment text as SQL.
DROP INDEX IF EXISTS index_app_version_on_app_id;

CREATE INDEX index_app_version_on_app_id ON app_version (app_id);

DROP INDEX IF EXISTS index_app_script_on_app;

CREATE INDEX index_app_script_on_app ON app_script (app);

DROP INDEX IF EXISTS index_workspace_runnable_dependencies_on_app_path;

CREATE INDEX index_workspace_runnable_dependencies_on_app_path
    ON workspace_runnable_dependencies (app_path, workspace_id) WHERE app_path IS NOT NULL;

DROP INDEX IF EXISTS index_workspace_runnable_dependencies_on_flow_path;

CREATE INDEX index_workspace_runnable_dependencies_on_flow_path
    ON workspace_runnable_dependencies (flow_path, workspace_id) WHERE flow_path IS NOT NULL;
