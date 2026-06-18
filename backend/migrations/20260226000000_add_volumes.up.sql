-- Add 'volume' to the asset_kind enum
ALTER TYPE asset_kind ADD VALUE IF NOT EXISTS 'volume';

-- Volume metadata table
CREATE TABLE volume (
    workspace_id VARCHAR(50) NOT NULL REFERENCES workspace(id) ON DELETE CASCADE,
    name VARCHAR(255) NOT NULL,
    size_bytes BIGINT NOT NULL DEFAULT 0,
    file_count INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    created_by VARCHAR(255) NOT NULL,
    updated_at TIMESTAMPTZ,
    updated_by VARCHAR(255),
    description TEXT NOT NULL DEFAULT '',
    lease_until TIMESTAMPTZ,
    leased_by VARCHAR(255),
    last_used_at TIMESTAMPTZ,
    extra_perms JSONB NOT NULL DEFAULT '{}',
    PRIMARY KEY (workspace_id, name)
);

CREATE INDEX idx_volume_last_used ON volume(workspace_id, last_used_at);
