-- SQL migrations defined per workspace for data tables.
-- `timestamp` is the migration version (YYYYMMDDHHMMSS), recorded as `version`
-- in the data table's `_wm_migrations` table once applied.
CREATE TABLE datatable_migrations (
    workspace_id VARCHAR(50) NOT NULL REFERENCES workspace(id) ON DELETE CASCADE,
    timestamp BIGINT NOT NULL,
    name VARCHAR(255) NOT NULL,
    code_up TEXT NOT NULL,
    code_down TEXT,
    PRIMARY KEY (workspace_id, timestamp)
);

-- No standalone index on workspace_id: the (workspace_id, timestamp) primary-key
-- btree already serves `WHERE workspace_id = $1` lookups via its leading column.

GRANT ALL ON datatable_migrations TO windmill_user;
GRANT ALL ON datatable_migrations TO windmill_admin;
