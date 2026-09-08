-- Catalog of `// measure` / `// dimension` declarations, one row per declaration,
-- synced from the producing script's annotations on deploy. Persisted (rather than
-- parsed from script content on demand) so folder-scoped listing is an index range
-- scan instead of a scan over every script body in the folder.
CREATE TABLE data_metric (
    workspace_id VARCHAR(50) NOT NULL REFERENCES workspace (id) ON UPDATE CASCADE ON DELETE CASCADE,
    -- The producing script. Also the permission anchor: ducklake table paths have
    -- no folder to authorize against, so reads are filtered on this.
    script_path VARCHAR(510) NOT NULL,
    table_path VARCHAR(510) NOT NULL,
    kind VARCHAR(16) NOT NULL CHECK (kind IN ('measure', 'dimension')),
    name VARCHAR(255) NOT NULL,
    expr TEXT NOT NULL,
    filter TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (workspace_id, script_path, kind, name)
);

-- "what does this table declare?" for the script editor drawer.
CREATE INDEX idx_data_metric_table ON data_metric (workspace_id, table_path);

-- Sort order for the listing endpoint's `ORDER BY table_path, kind, name,
-- script_path`, so each keyset page is an ordered index range scan resuming from
-- the previous page's last row rather than re-sorting the catalog every request.
CREATE INDEX idx_data_metric_page ON data_metric (workspace_id, table_path, kind, name, script_path);

-- "what is declared under this folder?" for the agent tool. text_pattern_ops so a
-- `LIKE 'f/folder/%'` prefix is a range scan: this database's collation is not C,
-- and under a locale collation the planner will not use a default-opclass index
-- for prefix matching.
CREATE INDEX idx_data_metric_folder ON data_metric (workspace_id, script_path text_pattern_ops);

-- Written on user_db transactions (SET LOCAL ROLE); the one-time GRANT ALL
-- migration predates this table, so explicit grants are required.
GRANT ALL ON data_metric TO windmill_user;
GRANT ALL ON data_metric TO windmill_admin;
