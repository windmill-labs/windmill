-- Rollback to the original clone_workspace_data function
-- (This would restore the version with the hash collision issue)
-- In practice, this rollback is not recommended as it would reintroduce the bug
DROP FUNCTION IF EXISTS clone_workspace_data(TEXT, TEXT, TEXT);