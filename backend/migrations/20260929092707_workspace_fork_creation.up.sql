-- A fork being created in the background, and how that ended. The fork's own workspace row only
-- exists once the copy commits, so this is where a failure is reported to whoever polls for it.
-- `heartbeat_at` is refreshed while the copy runs: an unfinished row that stops being refreshed
-- belongs to a server that went away mid-copy.
CREATE TABLE workspace_fork_creation (
    fork_workspace_id VARCHAR(50) PRIMARY KEY,
    parent_workspace_id VARCHAR(50) NOT NULL,
    created_by VARCHAR(255) NOT NULL,
    started_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    heartbeat_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    finished_at TIMESTAMPTZ,
    error TEXT
);

GRANT ALL ON workspace_fork_creation TO windmill_user;
GRANT ALL ON workspace_fork_creation TO windmill_admin;
