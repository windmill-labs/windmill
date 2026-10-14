-- One row per commit a workspace has come to reflect on a branch, written when a
-- pull of that commit succeeds or a deploy push produces it. The "Windmill CI
-- tests" PR check reads it to know when a workspace reflects a PR head, and
-- records the head's CI test runs on it. Kept apart from `workspace_settings.git_sync.auto_pull.last_synced_sha`,
-- which decides whether the next poll pulls and is client-round-tripped settings.
CREATE TABLE git_sync_synced_head (
    workspace_id VARCHAR(50) NOT NULL REFERENCES workspace(id) ON DELETE CASCADE,
    -- Repository resource path without its `$res:` prefix.
    repo_resource_path VARCHAR(255) NOT NULL,
    branch VARCHAR(255) NOT NULL,
    sha VARCHAR(64) NOT NULL,
    -- 'pull' rows name the pull job; 'push' rows the deploy push job.
    source VARCHAR(4) NOT NULL CHECK (source IN ('pull', 'push')),
    job_id UUID,
    synced_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    -- The head's own CI test suite, dispatched once the workspace reflects it and its
    -- dependency jobs settled: the "Windmill CI tests" check reads exactly these runs.
    tests_dispatched_at TIMESTAMPTZ,
    ci_test_job_ids UUID[],
    PRIMARY KEY (workspace_id, repo_resource_path, branch, sha)
);

GRANT ALL ON git_sync_synced_head TO windmill_user;
GRANT ALL ON git_sync_synced_head TO windmill_admin;
