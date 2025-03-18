ALTER TABLE workspace_runnable_dependencies
DROP COLUMN item_kind;

ALTER TABLE workspace_runnable_dependencies
RENAME COLUMN item_path TO flow_path;

ALTER TABLE workspace_runnable_dependencies
RENAME TO flow_workspace_runnables;

DROP TYPE RUNNABLE_DEPENDENCY_ITEM_KIND;