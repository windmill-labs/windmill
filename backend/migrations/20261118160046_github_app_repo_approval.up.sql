-- The repository URL a workspace admin last saved on an app-backed git_repository
-- resource. Anyone who can write the resource controls its URL, so the
-- installation only serves a non-admin a repository whose resolved URL an admin
-- approved here for that path.
CREATE TABLE github_app_repo_approval (
    workspace_id VARCHAR(50) NOT NULL REFERENCES workspace(id) ON DELETE CASCADE,
    resource_path VARCHAR(255) NOT NULL,
    url TEXT NOT NULL,
    approved_by VARCHAR(255) NOT NULL,
    approved_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (workspace_id, resource_path)
);

GRANT ALL ON github_app_repo_approval TO windmill_user;
GRANT ALL ON github_app_repo_approval TO windmill_admin;
