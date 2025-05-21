-- Add up migration script here
DROP INDEX IF EXISTS index_script_on_path_created_at;
CREATE INDEX IF NOT EXISTS index_script_on_path_created_at ON script (workspace_id, path, created_at DESC);
