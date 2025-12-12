-- Add up migration script here
CREATE TABLE workspace_diff (
    source_workspace_id VARCHAR(50) NOT NULL,
    fork_workspace_id VARCHAR(50) NOT NULL,
    path VARCHAR(255) NOT NULL,
    kind VARCHAR(50) NOT NULL,
    ahead INTEGER NOT NULL DEFAULT 0,
    behind INTEGER NOT NULL DEFAULT 0,
    has_changes BOOLEAN DEFAULT NULL,
    exists_in_source BOOLEAN DEFAULT NULL,
    exists_in_fork BOOLEAN DEFAULT NULL,
    PRIMARY KEY (source_workspace_id, fork_workspace_id, path, kind)
);

-- Create table to track workspaces that should be excluded from diff tallying
-- Old workspaces that are linked but have already diverged need to be skipped
CREATE TABLE skip_workspace_diff_tally (
    workspace_id VARCHAR(50) PRIMARY KEY,
    added_at TIMESTAMP NOT NULL DEFAULT NOW()
);

-- Populate with all existing workspaces to exclude them from new tallying logic
INSERT INTO skip_workspace_diff_tally (workspace_id)
SELECT id FROM workspace;
