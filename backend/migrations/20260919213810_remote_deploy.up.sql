-- The workspace on another Windmill instance this workspace deploys into from the UI:
-- `{"base_url": ..., "workspace_id": ...}`.
ALTER TABLE workspace_settings ADD COLUMN remote_deploy_target JSONB;

-- One user's token for the remote deploy target, encrypted with the workspace key.
-- `base_url`/`remote_workspace_id` name the target it was granted for: a token is only
-- ever sent to that target, so re-pointing the workspace setting cannot redirect it.
-- Keyed by email rather than by membership: a superadmin deploys from workspaces it is
-- not a member of, and the row is only ever reached by the same email signed in here.
CREATE TABLE remote_deploy_token (
    workspace_id VARCHAR(50) NOT NULL REFERENCES workspace(id) ON DELETE CASCADE,
    email VARCHAR(255) NOT NULL,
    base_url VARCHAR(1000) NOT NULL,
    remote_workspace_id VARCHAR(50) NOT NULL,
    token TEXT NOT NULL,
    remote_email VARCHAR(255) NOT NULL,
    connected_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (workspace_id, email)
);

GRANT ALL ON remote_deploy_token TO windmill_user;
GRANT ALL ON remote_deploy_token TO windmill_admin;
