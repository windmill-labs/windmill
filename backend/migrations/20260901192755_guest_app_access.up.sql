-- Guest app access: a workspace-level switch, off by default. An app whose policy says
-- `execution_mode: guest` only admits guests where this is on, and the check runs where
-- the guest session is minted -- an app definition carries its policy, so git-sync and
-- the CLI push `guest` past every UI gate.
ALTER TABLE workspace_settings
    ADD COLUMN guest_access_enabled BOOLEAN NOT NULL DEFAULT false;

-- A guest leaves no `usr` or `password` row, which is what keeps them off every seat
-- counter, so this is the only durable record that one was here: a row per guest,
-- workspace and day, written when the session is minted.
--
-- Deliberately not the audit log. The seat scan is served by a partial index whose
-- predicate names the login operations literally, and `audit_partitioned` is a
-- partitioned table, where `CREATE INDEX CONCURRENTLY` is unsupported -- adding a
-- guest operation to that predicate means a locking rebuild on the largest table an
-- instance has. Guest logins still write `users.login_guest` for the audit trail;
-- nothing counts them from there.
CREATE TABLE guest_activity (
    email VARCHAR(255) NOT NULL,
    workspace_id VARCHAR(50) NOT NULL,
    day DATE NOT NULL DEFAULT CURRENT_DATE,
    last_seen_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (email, workspace_id, day)
);

-- The retention delete filters on day alone; the PK only reaches it through two
-- other columns.
CREATE INDEX idx_guest_activity_day ON guest_activity (day);
