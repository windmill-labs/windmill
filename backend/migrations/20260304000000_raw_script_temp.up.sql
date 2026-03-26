-- Temporary storage for raw script content during CLI lock generation
-- Content is stored with hash as key, cleaned up after 1 week
CREATE TABLE raw_script_temp (
    workspace_id VARCHAR(50) NOT NULL REFERENCES workspace(id),
    hash CHAR(64) NOT NULL,
    content TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (workspace_id, hash)
);

CREATE INDEX IF NOT EXISTS idx_raw_script_temp_created_at ON raw_script_temp (created_at);
