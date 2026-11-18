-- The shares a member opened by link, which is what lists them in that member's sidebar.
-- Removing one from the list deletes its row; opening the link again brings it back.
CREATE TABLE ai_shared_session_opened (
    share_id UUID NOT NULL REFERENCES ai_shared_session(id) ON DELETE CASCADE,
    email VARCHAR(255) NOT NULL,
    opened_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (share_id, email)
);

GRANT ALL ON ai_shared_session_opened TO windmill_admin;
GRANT ALL ON ai_shared_session_opened TO windmill_user;

-- The handlers go through the raw pool and scope every query themselves; this is the backstop
-- for a future query that reaches the table through UserDB.
ALTER TABLE ai_shared_session_opened ENABLE ROW LEVEL SECURITY;

CREATE POLICY admin_policy ON ai_shared_session_opened FOR ALL TO windmill_admin USING (true);
