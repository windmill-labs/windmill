-- Fork review requests: a lightweight review + comments system on top of
-- the workspace fork/merge flow.
--
-- A review request represents one open conversation between a fork author
-- and one or more reviewers (admins or wm_deployers of the parent workspace).
-- There can only be one open review request per (source, fork) pair at a
-- time, enforced by the partial unique index below.

CREATE TABLE workspace_fork_review_request (
    id                  BIGSERIAL PRIMARY KEY,
    source_workspace_id VARCHAR(50) NOT NULL REFERENCES workspace(id) ON DELETE CASCADE,
    fork_workspace_id   VARCHAR(50) NOT NULL REFERENCES workspace(id) ON DELETE CASCADE,
    requested_by        VARCHAR(255) NOT NULL,
    requested_by_email  VARCHAR(255) NOT NULL,
    requested_at        TIMESTAMPTZ NOT NULL DEFAULT now(),
    closed_at           TIMESTAMPTZ,
    closed_reason       VARCHAR(20)
);

-- Only one open review request per (source, fork); closed requests are free to
-- accumulate for history.
CREATE UNIQUE INDEX workspace_fork_review_request_open_unique
    ON workspace_fork_review_request (source_workspace_id, fork_workspace_id)
    WHERE closed_at IS NULL;

CREATE INDEX workspace_fork_review_request_fork_idx
    ON workspace_fork_review_request (fork_workspace_id, closed_at);

-- Reviewers targeted by a request. Username + email captured at request time
-- so email dispatch doesn't depend on the user's current state in the
-- parent workspace.
CREATE TABLE workspace_fork_review_reviewer (
    request_id BIGINT NOT NULL REFERENCES workspace_fork_review_request(id) ON DELETE CASCADE,
    username   VARCHAR(255) NOT NULL,
    email      VARCHAR(255) NOT NULL,
    PRIMARY KEY (request_id, username)
);

-- Comments on a review request. `parent_id` threads replies under a top-level
-- comment. `anchor_kind`/`anchor_path` pin a comment to a specific diff row;
-- both NULL = general comment about the whole fork. Anchored comments are
-- flipped to obsolete = true when the underlying item is updated in the fork,
-- and all comments on a request are marked obsolete on successful merge.
CREATE TABLE workspace_fork_review_comment (
    id           BIGSERIAL PRIMARY KEY,
    request_id   BIGINT NOT NULL REFERENCES workspace_fork_review_request(id) ON DELETE CASCADE,
    parent_id    BIGINT REFERENCES workspace_fork_review_comment(id) ON DELETE CASCADE,
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

CREATE INDEX workspace_fork_review_comment_request_idx
    ON workspace_fork_review_comment (request_id);

-- Anchored-comment lookup: find the comments on an open request that pin
-- to a specific (kind, path) — used when we mark comments obsolete after
-- an item in the fork is updated.
CREATE INDEX workspace_fork_review_comment_anchor_idx
    ON workspace_fork_review_comment (request_id, anchor_kind, anchor_path)
    WHERE anchor_kind IS NOT NULL;
