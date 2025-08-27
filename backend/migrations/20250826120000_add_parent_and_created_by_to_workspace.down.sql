-- Rollback: Remove parent_workspace_id and created_by columns from workspace table

-- Drop the index first
DROP INDEX IF EXISTS workspace_parent_idx;

-- Remove the columns
ALTER TABLE workspace DROP COLUMN IF EXISTS parent_workspace_id;
ALTER TABLE workspace DROP COLUMN IF EXISTS created_by;
ALTER TABLE workspace DROP COLUMN IF EXISTS created_at;