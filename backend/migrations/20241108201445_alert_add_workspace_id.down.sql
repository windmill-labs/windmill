ALTER TABLE alerts DROP COLUMN workspace_id;
ALTER TABLE alerts DROP COLUMN acknowledged_workspace;
ALTER TABLE alerts DROP COLUMN resource;
ALTER TABLE workspace_settings DROP COLUMN mute_critical_alerts;
