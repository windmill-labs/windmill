-- Migrate native trigger OAuth tokens from workspace_integrations to account+variable+resource pattern

-- Add flag to distinguish workspace integration accounts from regular user OAuth accounts
ALTER TABLE account ADD COLUMN is_workspace_integration BOOLEAN NOT NULL DEFAULT false;

-- Make oauth_data nullable since it will only store client config (no tokens) going forward
ALTER TABLE workspace_integrations ALTER COLUMN oauth_data DROP NOT NULL;

-- Add resource_path column to workspace_integrations
ALTER TABLE workspace_integrations ADD COLUMN IF NOT EXISTS resource_path TEXT;
