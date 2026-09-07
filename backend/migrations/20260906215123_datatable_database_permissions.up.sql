-- Role-based access to a data table's database is a property of the Postgres
-- database the roles are created in, not of the config entry that points at it:
-- every entry reaching the same database, in any workspace, resolves to this row.
-- database_key: 'instance:<dbname>' for an instance database, 'pg:<sha256 of
-- host, port and dbname>' for a resource-backed one. The tenants named in
-- `permissions` are principals of owner_workspace_id, and only its admins manage
-- the row.
CREATE TABLE datatable_database_permissions (
    database_key TEXT PRIMARY KEY,
    owner_workspace_id VARCHAR(50) NOT NULL REFERENCES workspace(id) ON DELETE CASCADE,
    permissions JSONB NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX datatable_database_permissions_owner ON datatable_database_permissions (owner_workspace_id);

GRANT ALL ON datatable_database_permissions TO windmill_user;
GRANT ALL ON datatable_database_permissions TO windmill_admin;
