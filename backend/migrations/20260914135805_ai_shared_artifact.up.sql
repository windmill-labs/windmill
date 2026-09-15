-- A copy of an AI session artifact that its author explicitly shared with the workspace.
-- Artifacts otherwise live only in the author's browser; this row exists only while the
-- share does, and the monitor deletes it once `shared_at` falls outside
-- AI_SHARED_ARTIFACT_RETENTION_SECS.
CREATE TABLE ai_shared_artifact (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    workspace_id VARCHAR(50) NOT NULL REFERENCES workspace(id) ON DELETE CASCADE,
    -- The browser-side artifact id. Unique per author so sharing the same artifact again
    -- moves its one link forward rather than minting a second one.
    artifact_id VARCHAR(255) NOT NULL,
    email VARCHAR(255) NOT NULL,
    created_by VARCHAR(255) NOT NULL,
    name VARCHAR(255) NOT NULL,
    kind VARCHAR(10) NOT NULL CHECK (kind IN ('md', 'html')),
    version INTEGER NOT NULL,
    content TEXT NOT NULL,
    -- Reset on every re-share: retention counts from the last time the author shared it.
    shared_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (workspace_id, email, artifact_id)
);

CREATE INDEX idx_ai_shared_artifact_shared_at ON ai_shared_artifact (shared_at);

GRANT ALL ON ai_shared_artifact TO windmill_admin;
GRANT ALL ON ai_shared_artifact TO windmill_user;

-- The handlers go through the raw pool and scope every query to the workspace themselves.
-- An admin-only policy is the backstop for a future query that reaches this table through
-- UserDB.
ALTER TABLE ai_shared_artifact ENABLE ROW LEVEL SECURITY;

CREATE POLICY admin_policy ON ai_shared_artifact FOR ALL TO windmill_admin USING (true);
