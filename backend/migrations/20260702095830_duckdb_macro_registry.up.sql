-- Workspace DuckDB macro registry: one row per macro defined by a deployed
-- `// macros` library script. Names are workspace-unique (macros are injected
-- unqualified into consumer jobs).
CREATE TABLE macro_definition (
    workspace_id VARCHAR(50) NOT NULL REFERENCES workspace (id) ON DELETE CASCADE,
    name VARCHAR(255) NOT NULL,
    provider_path VARCHAR(510) NOT NULL,
    params TEXT NOT NULL,
    body TEXT NOT NULL,
    is_table_macro BOOLEAN NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (workspace_id, name)
);
CREATE INDEX idx_macro_definition_provider ON macro_definition (workspace_id, provider_path);

-- Deploy-recorded consumer→macro edges, for the asset graph only (the worker
-- re-detects calls live at job time).
CREATE TABLE macro_usage (
    workspace_id VARCHAR(50) NOT NULL REFERENCES workspace (id) ON DELETE CASCADE,
    consumer_path VARCHAR(510) NOT NULL,
    macro_name VARCHAR(255) NOT NULL,
    PRIMARY KEY (workspace_id, consumer_path, macro_name)
);
CREATE INDEX idx_macro_usage_name ON macro_usage (workspace_id, macro_name);

-- Both tables are written on user_db transactions (SET LOCAL ROLE); the
-- one-time GRANT ALL migration predates them, so explicit grants are required.
GRANT ALL ON macro_definition TO windmill_user;
GRANT ALL ON macro_definition TO windmill_admin;
GRANT ALL ON macro_usage TO windmill_user;
GRANT ALL ON macro_usage TO windmill_admin;
