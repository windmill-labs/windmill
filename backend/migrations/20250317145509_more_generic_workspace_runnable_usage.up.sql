-- flow_workspace_runnables only stored runnable usages by
-- flows although apps can also use runnables

CREATE TYPE RUNNABLE_DEPENDENCY_ITEM_KIND AS ENUM ('flow', 'app');

ALTER TABLE flow_workspace_runnables
RENAME TO workspace_runnable_dependencies;

ALTER TABLE workspace_runnable_dependencies
RENAME COLUMN flow_path TO item_path;

ALTER TABLE workspace_runnable_dependencies
ADD COLUMN item_kind RUNNABLE_DEPENDENCY_ITEM_KIND;

UPDATE workspace_runnable_dependencies
SET item_kind = 'flow';