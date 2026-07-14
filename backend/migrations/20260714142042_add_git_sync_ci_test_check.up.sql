-- One "Windmill CI tests" GitHub check run per (workspace, deployed commit): a
-- git-sync deploy opens the check in_progress and it is concluded once the CI
-- tests triggered by that deploy settle, so the results can gate a GitHub PR.
CREATE TABLE git_sync_ci_test_check (
    -- The fork workspace whose CI tests gate the PR: keys the row, and its `ci_test`
    -- jobs are what the check reflects (a fork inherits no git_app_installations).
    workspace_id VARCHAR(50) NOT NULL REFERENCES workspace(id),
    head_sha VARCHAR(64) NOT NULL,
    -- The workspace whose GitHub App installation posts the check run (the parent that
    -- owns the repo webhook); forks can't mint the token themselves.
    github_workspace_id VARCHAR(50) NOT NULL REFERENCES workspace(id),
    repo_url TEXT NOT NULL,
    -- NULL when the GitHub check-run creation failed; the poller retries the create.
    check_run_id BIGINT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    concluded BOOLEAN NOT NULL DEFAULT false,
    conclusion TEXT,
    concluded_at TIMESTAMPTZ,
    -- Decoupled from `concluded` so a failed check-run PATCH is retried by the
    -- poller instead of hanging a required check on GitHub.
    github_posted BOOLEAN NOT NULL DEFAULT false,
    PRIMARY KEY (workspace_id, head_sha)
);

-- Rows still needing action (create retry, conclusion, timeout, delivery retry).
-- A row drops out only once it is both concluded and delivered to GitHub, so the
-- per-job conclusion hook and the poller sweeper both scan a small live set.
CREATE INDEX idx_git_sync_ci_test_check_pending
    ON git_sync_ci_test_check (workspace_id)
    WHERE NOT concluded OR NOT github_posted;

GRANT ALL ON git_sync_ci_test_check TO windmill_user;
GRANT ALL ON git_sync_ci_test_check TO windmill_admin;
