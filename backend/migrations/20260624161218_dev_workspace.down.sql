DROP INDEX IF EXISTS workspace_canonical_dev_idx;
ALTER TABLE workspace DROP COLUMN is_dev_workspace;
