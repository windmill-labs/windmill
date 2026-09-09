-- One "Windmill CI tests" GitHub check run per (fork workspace, repository, PR head commit):
-- the pull_request webhook opens the check in_progress and it is concluded once
-- the fork's CI tests settle, so the results can gate a GitHub PR.
CREATE TABLE git_sync_ci_test_check (
    -- The fork workspace whose CI tests gate the PR: keys the row, and its `ci_test`
    -- jobs are what the check reflects.
    workspace_id VARCHAR(50) NOT NULL REFERENCES workspace(id) ON DELETE CASCADE,
    head_sha VARCHAR(64) NOT NULL,
    -- The PR's head branch: the check waits until the fork's synced state for this
    -- branch (written by its pushes and pulls alike) names `head_sha`.
    head_ref VARCHAR(255) NOT NULL,
    -- The workspace whose GitHub App installation posts the check run: the one that
    -- received the pull_request webhook (the parent owning the repo hook).
    github_workspace_id VARCHAR(50) NOT NULL REFERENCES workspace(id) ON DELETE CASCADE,
    repo_url TEXT NOT NULL,
    -- The fork's copy of the repository resource: keys the synced-head lookup, since a
    -- fork syncing two repositories names its branch identically in both.
    repo_resource_path VARCHAR(255) NOT NULL,
    -- NULL when the GitHub check-run creation failed; the poller retries the create.
    check_run_id BIGINT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    concluded BOOLEAN NOT NULL DEFAULT false,
    conclusion TEXT,
    concluded_at TIMESTAMPTZ,
    -- Decoupled from `concluded` so a failed check-run PATCH is retried by the
    -- poller instead of hanging a required check on GitHub.
    github_posted BOOLEAN NOT NULL DEFAULT false,
    PRIMARY KEY (workspace_id, repo_resource_path, head_sha)
);

-- Rows still needing action (create retry, conclusion, timeout, delivery retry).
-- A row drops out only once it is both concluded and delivered to GitHub, so the
-- per-job conclusion hook and the poller sweeper both scan a small live set.
CREATE INDEX idx_git_sync_ci_test_check_pending
    ON git_sync_ci_test_check (workspace_id)
    WHERE NOT concluded OR NOT github_posted;

GRANT ALL ON git_sync_ci_test_check TO windmill_user;
GRANT ALL ON git_sync_ci_test_check TO windmill_admin;
