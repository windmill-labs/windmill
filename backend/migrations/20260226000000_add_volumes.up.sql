-- Add 'volume' to the asset_kind enum
ALTER TYPE asset_kind ADD VALUE IF NOT EXISTS 'volume';

-- Volume metadata table
CREATE TABLE volume (
    workspace_id VARCHAR(50) NOT NULL REFERENCES workspace(id),
    name VARCHAR(255) NOT NULL,
    size_bytes BIGINT NOT NULL DEFAULT 0,
    file_count INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    created_by VARCHAR(255) NOT NULL,
    extra_perms JSONB NOT NULL DEFAULT '{}',
    lease_until TIMESTAMPTZ,
    leased_by VARCHAR(255),
    last_used_at TIMESTAMPTZ,
    PRIMARY KEY (workspace_id, name)
);

