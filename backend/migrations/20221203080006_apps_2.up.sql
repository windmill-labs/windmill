-- Add up migration script here
ALTER TABLE app ADD CONSTRAINT unique_path_workspace_id UNIQUE (workspace_id, path);
ALTER TABLE app ENABLE ROW LEVEL SECURITY;
