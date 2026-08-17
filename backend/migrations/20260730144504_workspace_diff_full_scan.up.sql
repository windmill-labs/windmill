-- Records that a (source, fork) pair outside the fork lineage had its
-- `workspace_diff` candidate set seeded by an explicit full scan. Nothing tallies
-- such a pair, so without this marker `compare_workspaces` cannot tell "no
-- differences" from "never computed".
CREATE TABLE workspace_diff_full_scan (
    source_workspace_id VARCHAR(50) NOT NULL,
    fork_workspace_id VARCHAR(50) NOT NULL,
    scanned_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (source_workspace_id, fork_workspace_id)
);

GRANT ALL ON workspace_diff_full_scan TO windmill_user;
GRANT ALL ON workspace_diff_full_scan TO windmill_admin;
