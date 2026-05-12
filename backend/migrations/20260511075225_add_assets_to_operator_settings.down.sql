-- Remove "assets" key from operator_settings
UPDATE workspace_settings
SET operator_settings = operator_settings - 'assets'
WHERE operator_settings IS NOT NULL
  AND operator_settings ? 'assets';

-- Revert the column default
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
    "audit_logs": true
}';
