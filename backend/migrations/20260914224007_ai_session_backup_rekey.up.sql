-- A workspace key rotation records the key it replaced here; the read path of the AI
-- session backups decrypts with these keys too, and the re-key walk notes each storage it
-- has rewritten every object of (a workspace may point at several over time).
CREATE TABLE ai_session_backup_rekey (
    workspace_id VARCHAR(50) NOT NULL REFERENCES workspace(id) ON DELETE CASCADE,
    previous_key VARCHAR(255) NOT NULL,
    started_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    walked_storages TEXT[] NOT NULL DEFAULT '{}',
    PRIMARY KEY (workspace_id, previous_key)
);
