-- Fork deployment requests: a lightweight request + comments layer on top
-- of the workspace fork/merge flow.
--
-- When a fork author can't deploy to the parent workspace (because the
-- RestrictDeployToDeployers rule is active and they aren't admin / wm_deployers),
-- they open a deployment request naming one or more eligible deployers as
-- assignees. Assignees comment, the chosen deployer performs the merge,
-- and the request auto-closes on success. There can only be one open
-- request per (source, fork) pair — enforced by the partial unique index.

CREATE TABLE workspace_fork_deployment_request (
    id                  BIGSERIAL PRIMARY KEY,
    source_workspace_id VARCHAR(50) NOT NULL REFERENCES workspace(id) ON DELETE CASCADE,
    fork_workspace_id   VARCHAR(50) NOT NULL REFERENCES workspace(id) ON DELETE CASCADE,
    requested_by        VARCHAR(255) NOT NULL,
    requested_by_email  VARCHAR(255) NOT NULL,
    requested_at        TIMESTAMPTZ NOT NULL DEFAULT now(),
    closed_at           TIMESTAMPTZ,
    closed_reason       VARCHAR(20)
);

-- Only one open deployment request per (source, fork). Closed requests
-- accumulate freely for history.
CREATE UNIQUE INDEX workspace_fork_deployment_request_open_unique
    ON workspace_fork_deployment_request (source_workspace_id, fork_workspace_id)
    WHERE closed_at IS NULL;

CREATE INDEX workspace_fork_deployment_request_fork_idx
    ON workspace_fork_deployment_request (fork_workspace_id, closed_at);

-- Assignees for a deployment request — the users asked to merge. Username
-- + email are snapshotted at request creation so notification dispatch
-- doesn't depend on the user's current workspace membership.
CREATE TABLE workspace_fork_deployment_request_assignee (
    request_id BIGINT NOT NULL REFERENCES workspace_fork_deployment_request(id) ON DELETE CASCADE,
    username   VARCHAR(255) NOT NULL,
    email      VARCHAR(255) NOT NULL,
    PRIMARY KEY (request_id, username)
);

-- Comments on a deployment request. `parent_id` threads replies under a
-- top-level comment (2 levels max, enforced in application code).
-- `anchor_kind`/`anchor_path` pin a comment to a specific diff row; both
-- NULL = general comment about the whole fork. Anchored comments flip to
-- obsolete when the underlying item is updated in the fork, and all
-- comments on a request are marked obsolete on successful merge.
CREATE TABLE workspace_fork_deployment_request_comment (
    id           BIGSERIAL PRIMARY KEY,
    request_id   BIGINT NOT NULL REFERENCES workspace_fork_deployment_request(id) ON DELETE CASCADE,
    parent_id    BIGINT REFERENCES workspace_fork_deployment_request_comment(id) ON DELETE CASCADE,
    author       VARCHAR(255) NOT NULL,
    author_email VARCHAR(255) NOT NULL,
    body         TEXT NOT NULL,
    anchor_kind  VARCHAR(50),
    anchor_path  VARCHAR(255),
    obsolete     BOOLEAN NOT NULL DEFAULT false,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT anchor_kind_path_both_or_neither
        CHECK ((anchor_kind IS NULL) = (anchor_path IS NULL))
);

CREATE INDEX workspace_fork_deployment_request_comment_request_idx
    ON workspace_fork_deployment_request_comment (request_id);

-- Anchored-comment lookup: find all open-request comments pinned to a
-- specific (kind, path) — used when we mark comments obsolete after an
-- item in the fork is updated.
CREATE INDEX workspace_fork_deployment_request_comment_anchor_idx
    ON workspace_fork_deployment_request_comment (request_id, anchor_kind, anchor_path)
    WHERE anchor_kind IS NOT NULL;
