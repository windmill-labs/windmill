ALTER TABLE workspace DROP CONSTRAINT IF EXISTS workspace_dev_requires_parent;
DROP INDEX IF EXISTS workspace_canonical_dev_idx;
ALTER TABLE workspace DROP COLUMN is_dev_workspace;
