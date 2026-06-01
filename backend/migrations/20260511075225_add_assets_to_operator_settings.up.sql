-- Add "assets": true to operator_settings for all workspaces that have operator_settings
-- but don't already have an "assets" key
UPDATE workspace_settings
SET operator_settings = operator_settings || '{"assets": true}'::jsonb
WHERE operator_settings IS NOT NULL
  AND NOT operator_settings ? 'assets';

-- Update the column default to include assets
ALTER TABLE workspace_settings
ALTER COLUMN operator_settings SET DEFAULT '{
    "runs": true,
    "groups": true,
    "folders": true,
    "workers": true,
    "triggers": true,
    "resources": true,
    "schedules": true,
    "variables": true,
    "audit_logs": true,
    "assets": true
}';
