-- Add up migration script here
CREATE TABLE workspace_shared_ui (
    workspace_id VARCHAR(50) PRIMARY KEY REFERENCES workspace(id) ON DELETE CASCADE,
    files JSONB NOT NULL DEFAULT '{}'::jsonb,
    version BIGINT NOT NULL DEFAULT 0,
    edited_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    edited_by VARCHAR(255) NOT NULL DEFAULT ''
);

GRANT ALL ON workspace_shared_ui TO windmill_user;
GRANT ALL ON workspace_shared_ui TO windmill_admin;
