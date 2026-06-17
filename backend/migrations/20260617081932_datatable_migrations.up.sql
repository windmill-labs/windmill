-- SQL migrations defined per data table within a workspace.
-- `datatable` is the target data table name, `name` is the migration name
-- (e.g. add_index_to_customers), and `timestamp` is the migration version
-- (YYYYMMDDHHMMSS), recorded as `version` in the data table's `_wm_migrations`
-- table once applied.
CREATE TABLE datatable_migrations (
    workspace_id VARCHAR(50) NOT NULL REFERENCES workspace(id) ON DELETE CASCADE,
    datatable VARCHAR(255) NOT NULL,
    timestamp BIGINT NOT NULL,
    name VARCHAR(255) NOT NULL,
    code_up TEXT NOT NULL,
    code_down TEXT,
    PRIMARY KEY (workspace_id, datatable, timestamp)
);

-- No standalone index: the (workspace_id, datatable, timestamp) primary-key btree
-- already serves both `WHERE workspace_id = $1` and `WHERE workspace_id = $1 AND
-- datatable = $2` lookups via its leading columns.

GRANT ALL ON datatable_migrations TO windmill_user;
GRANT ALL ON datatable_migrations TO windmill_admin;
