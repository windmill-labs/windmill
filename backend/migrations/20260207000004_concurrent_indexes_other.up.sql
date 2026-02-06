-- audit, alerts, script, log_file: drop obsolete indexes and create new ones CONCURRENTLY
DROP INDEX CONCURRENTLY IF EXISTS log_file_hostname_log_ts_idx;

CREATE INDEX CONCURRENTLY IF NOT EXISTS ix_audit_timestamps
    ON audit (timestamp DESC);

CREATE INDEX CONCURRENTLY IF NOT EXISTS alerts_by_workspace
    ON alerts (workspace_id);

CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_audit_recent_login_activities
    ON audit (timestamp, username)
    WHERE operation IN ('users.login', 'oauth.login', 'users.token.refresh');

CREATE INDEX CONCURRENTLY IF NOT EXISTS script_not_archived
    ON script (workspace_id, path, created_at DESC)
    WHERE archived = false;
