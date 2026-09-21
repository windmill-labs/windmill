-- The workspace on another Windmill instance this workspace deploys into from the UI:
-- `{"base_url": ..., "workspace_id": ...}`, and when it last changed. A token connected before
-- that change counts for nothing, even once the setting points back at the target it was for.
ALTER TABLE workspace_settings
    ADD COLUMN remote_deploy_target JSONB,
    ADD COLUMN remote_deploy_target_changed_at TIMESTAMPTZ;

-- One user's token for the remote deploy target, encrypted with the workspace key.
-- `base_url`/`remote_workspace_id` name the target it was granted for: a token is only
-- ever sent to that target, so re-pointing the workspace setting cannot redirect it.
-- Keyed by the account rather than by membership, since a superadmin deploys from workspaces
-- it is not a member of. The account key cascades: whatever deletes or renames an account
-- takes its tokens along, so a later account with the same address cannot inherit them.
CREATE TABLE remote_deploy_token (
    workspace_id VARCHAR(50) NOT NULL REFERENCES workspace(id) ON DELETE CASCADE,
    email VARCHAR(255) NOT NULL REFERENCES password(email) ON DELETE CASCADE ON UPDATE CASCADE,
    base_url VARCHAR(1000) NOT NULL,
    remote_workspace_id VARCHAR(50) NOT NULL,
    token TEXT NOT NULL,
    remote_email VARCHAR(255) NOT NULL,
    -- Part of every proxy URL, and readable only by its owner through the API: a link from
    -- elsewhere, which rides the session cookie, cannot know it and so cannot spend the token.
    proxy_key VARCHAR(64) NOT NULL,
    connected_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (workspace_id, email)
);

GRANT ALL ON remote_deploy_token TO windmill_user;
GRANT ALL ON remote_deploy_token TO windmill_admin;
