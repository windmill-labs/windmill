-- The last pull/merge request head the "Windmill diff" check ran for, per git-sync
-- repository. The webhook receiver and the auto-pull poller both claim a head here
-- before checking it, so each head is checked once whichever path sees it first.
CREATE TABLE git_sync_pr_check_head (
    workspace_id VARCHAR(50) NOT NULL REFERENCES workspace(id) ON DELETE CASCADE,
    repo_resource_path VARCHAR(255) NOT NULL,
    pr_number BIGINT NOT NULL,
    head_sha VARCHAR(64) NOT NULL,
    checked_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    -- Whether the check was started. A claim whose check never started (an error,
    -- or a restart in between) may be claimed again after a delay.
    started BOOLEAN NOT NULL DEFAULT false,
    PRIMARY KEY (workspace_id, repo_resource_path, pr_number)
);

GRANT ALL ON git_sync_pr_check_head TO windmill_user;
GRANT ALL ON git_sync_pr_check_head TO windmill_admin;
