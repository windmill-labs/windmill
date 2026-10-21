-- A read-only snapshot of an AI session transcript that its author shared by link. Sessions
-- otherwise live only in the author's browser; this row exists only while the share does.
-- Non-public shares are readable by members of the workspace, public ones by anyone with the
-- link, which is why `id` is a random UUID and never derived from the session.
CREATE TABLE ai_shared_session (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    workspace_id VARCHAR(50) NOT NULL REFERENCES workspace(id) ON DELETE CASCADE,
    -- The browser-side session id. Unique per author so sharing the same session again
    -- moves its one link forward rather than minting a second one.
    session_id VARCHAR(255) NOT NULL,
    email VARCHAR(255) NOT NULL,
    created_by VARCHAR(255) NOT NULL,
    name VARCHAR(255) NOT NULL,
    is_public BOOLEAN NOT NULL DEFAULT false,
    message_count INTEGER NOT NULL,
    -- JSON kept as text: jsonb refuses the \u0000 escape a transcript can carry.
    snapshot TEXT NOT NULL,
    shared_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (workspace_id, email, session_id)
);

GRANT ALL ON ai_shared_session TO windmill_admin;
GRANT ALL ON ai_shared_session TO windmill_user;

-- The handlers go through the raw pool and scope every query to the workspace themselves.
-- An admin-only policy is the backstop for a future query that reaches this table through
-- UserDB.
ALTER TABLE ai_shared_session ENABLE ROW LEVEL SECURITY;

CREATE POLICY admin_policy ON ai_shared_session FOR ALL TO windmill_admin USING (true);
