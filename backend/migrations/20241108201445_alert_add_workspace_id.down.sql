ALTER TABLE alerts DROP COLUMN workspace_id;
ALTER TABLE alerts RENAME COLUMN acknowledged_global TO acknowledged;
ALTER TABLE alerts DROP COLUMN acknowledged_workspace;
ALTER TABLE workspace_settings DROP COLUMN mute_critical_alerts;
