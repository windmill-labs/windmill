-- Datatable migrations: versioned SQL scripts attached to a datatable.
-- These are the source of truth (synced to git via the CLI as .sql files under
-- _datatable_migrations/) and are applied to the datatable's backing database,
-- with applied state tracked inside that database in a _windmill_datatable_migrations table.
CREATE TABLE IF NOT EXISTS datatable_migration (
    workspace_id VARCHAR(50) NOT NULL REFERENCES workspace(id) ON DELETE CASCADE,
    datatable TEXT NOT NULL,
    version TEXT NOT NULL,
    name TEXT NOT NULL DEFAULT '',
    content TEXT NOT NULL,
    checksum TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    created_by VARCHAR(255) NOT NULL DEFAULT '',
    PRIMARY KEY (workspace_id, datatable, version)
);

CREATE INDEX IF NOT EXISTS datatable_migration_ws_dt_idx
    ON datatable_migration (workspace_id, datatable);
