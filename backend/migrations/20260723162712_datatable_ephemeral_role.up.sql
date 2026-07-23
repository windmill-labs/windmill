-- Bookkeeping for ephemeral per-caller Postgres roles created for
-- permissions-enabled data tables. The actual roles live on the data table's
-- Postgres cluster; this table (on the main DB) tracks their passwords
-- (workspace-key encrypted), the hash of the grants they were created with,
-- and a sliding expiry.
CREATE TABLE datatable_ephemeral_role (
    role_name TEXT PRIMARY KEY,
    workspace_id VARCHAR(50) NOT NULL REFERENCES workspace(id) ON DELETE CASCADE,
    datatable VARCHAR(255) NOT NULL,
    permissioned_as VARCHAR(255) NOT NULL,
    password TEXT NOT NULL,
    perms_hash TEXT NOT NULL,
    expires_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX datatable_ephemeral_role_expires_at_idx ON datatable_ephemeral_role (expires_at);
CREATE INDEX datatable_ephemeral_role_workspace_idx ON datatable_ephemeral_role (workspace_id, datatable);

GRANT ALL ON datatable_ephemeral_role TO windmill_user;
GRANT ALL ON datatable_ephemeral_role TO windmill_admin;
