ALTER TABLE alerts ADD COLUMN workspace_id TEXT DEFAULT NULL;
ALTER TABLE workspace_settings ADD COLUMN mute_critical_alerts BOOL DEFAULT NULL;