-- Dedicated per-database owner role for permissions-enabled instance data
-- tables. The shared `custom_instance_user` is handed to user SQL on every
-- unprotected instance data table and holds CONNECT on the whole cluster, so a
-- non-admin could set its password (`ALTER ROLE CURRENT_USER PASSWORD`) and
-- reconnect to a protected database as the owner, bypassing the per-caller
-- grants. Protected databases therefore get an owner role that is never handed
-- to non-admin SQL, and `custom_instance_user` loses CONNECT on them.
CREATE TABLE datatable_owner_role (
    dbname TEXT PRIMARY KEY,
    role_name TEXT NOT NULL,
    -- Encrypted with the workspace key of `workspace_id`, like the ephemeral
    -- role passwords. Deprovisioned (ownership reassigned back) whenever the
    -- data table, its permissions or its workspace go away, so the row never
    -- outlives the key that decrypts it.
    password TEXT NOT NULL,
    workspace_id VARCHAR(50) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX datatable_owner_role_workspace_idx ON datatable_owner_role (workspace_id);

GRANT ALL ON datatable_owner_role TO windmill_user;
GRANT ALL ON datatable_owner_role TO windmill_admin;
