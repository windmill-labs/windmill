CREATE TABLE trashbin (
    id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    workspace_id VARCHAR(50) NOT NULL REFERENCES workspace(id),
    item_kind VARCHAR(50) NOT NULL,
    item_path VARCHAR(255) NOT NULL,
    item_data JSONB NOT NULL,
    deleted_by VARCHAR(255) NOT NULL,
    deleted_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    expires_at TIMESTAMPTZ NOT NULL DEFAULT now() + INTERVAL '3 days'
);

CREATE INDEX idx_trashbin_workspace_expires ON trashbin(workspace_id, expires_at);
CREATE INDEX idx_trashbin_workspace_kind ON trashbin(workspace_id, item_kind);
