-- An item a user hid from the workspace home's "Recently run" panel: its runs leave that panel's
-- list and counts for that user only.
CREATE TABLE recent_run_hidden_item (
    workspace_id VARCHAR(50) NOT NULL REFERENCES workspace(id) ON DELETE CASCADE,
    email VARCHAR(255) NOT NULL,
    kind VARCHAR(10) NOT NULL CHECK (kind IN ('script', 'flow', 'agent')),
    path VARCHAR(255) NOT NULL,
    hidden_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (workspace_id, email, kind, path)
);

GRANT ALL ON recent_run_hidden_item TO windmill_admin;
GRANT ALL ON recent_run_hidden_item TO windmill_user;

-- The handlers go through the raw pool and scope every query to the caller's own rows. An
-- admin-only policy is the backstop for a future query that reaches this table through UserDB.
ALTER TABLE recent_run_hidden_item ENABLE ROW LEVEL SECURITY;

CREATE POLICY admin_policy ON recent_run_hidden_item FOR ALL TO windmill_admin USING (true);
