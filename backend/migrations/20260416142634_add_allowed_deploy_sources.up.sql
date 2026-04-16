-- Add allowed_deploy_sources column to workspace_protection_rule
-- When DisableDirectDeployment is active and this array is non-empty,
-- only deployments from the listed sources (ui, cli, api) are allowed.
-- Empty array means all sources are blocked (backward compatible).
ALTER TABLE workspace_protection_rule
    ADD COLUMN IF NOT EXISTS allowed_deploy_sources TEXT[] NOT NULL DEFAULT '{}';
