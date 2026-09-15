-- Bumped by every workspace key rotation: the AI session backups in the workspace storage
-- live under a prefix named by it, so a rotation moves to a fresh prefix and the previous
-- ones can be deleted at leisure without ever touching live objects.
ALTER TABLE workspace_settings ADD COLUMN ai_sessions_backup_generation BIGINT NOT NULL DEFAULT 0;
