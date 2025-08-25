ALTER TABLE workspace_settings ADD COLUMN operator_settings JSONB DEFAULT '{
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
