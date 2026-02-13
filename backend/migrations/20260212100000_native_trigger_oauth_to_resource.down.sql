ALTER TABLE account DROP COLUMN IF EXISTS is_workspace_integration;
ALTER TABLE workspace_integrations ALTER COLUMN oauth_data SET NOT NULL;
ALTER TABLE workspace_integrations DROP COLUMN IF EXISTS resource_path;