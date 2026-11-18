-- The repository URL a workspace admin last saved on an app-backed git_repository
-- resource. Anyone who can write the resource controls its URL, so the GitHub App
-- installation serves a non-admin only the repository this names. The cascading FK
-- carries it through every rename and takes it with a delete, so a new resource at
-- a freed path never inherits an approval.
CREATE TABLE github_app_repo_approval (
    workspace_id VARCHAR(50) NOT NULL,
    path VARCHAR(255) NOT NULL,
    url TEXT NOT NULL,
    PRIMARY KEY (workspace_id, path),
    FOREIGN KEY (workspace_id, path) REFERENCES resource (workspace_id, path)
        ON DELETE CASCADE ON UPDATE CASCADE
);

GRANT ALL ON github_app_repo_approval TO windmill_user;
GRANT ALL ON github_app_repo_approval TO windmill_admin;
