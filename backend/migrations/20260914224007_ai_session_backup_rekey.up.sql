-- A workspace key rotation records the key it replaced here until every AI session backup
-- object in the workspace storage has been re-keyed; the read path decrypts with these
-- keys too in the meantime.
CREATE TABLE ai_session_backup_rekey (
    workspace_id VARCHAR(50) NOT NULL REFERENCES workspace(id) ON DELETE CASCADE,
    previous_key VARCHAR(255) NOT NULL,
    started_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (workspace_id, previous_key)
);
