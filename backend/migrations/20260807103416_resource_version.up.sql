-- Add up migration script here

-- Append-only value history for resources. Keyed by (workspace_id, path) with a
-- cascading FK so a rename carries the history along and a delete takes it with it;
-- without ON UPDATE CASCADE a rename would strand the history at the old path.
CREATE TABLE resource_version (
    id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    workspace_id VARCHAR(50) NOT NULL,
    path VARCHAR(255) NOT NULL,
    resource_type VARCHAR(50) NOT NULL,
    value JSONB,
    created_by VARCHAR(500),
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT now(),
    FOREIGN KEY (workspace_id, path) REFERENCES resource (workspace_id, path)
        ON DELETE CASCADE ON UPDATE CASCADE
);

-- Serves both "latest version of this path" and the paginated history listing.
CREATE INDEX index_resource_version_path ON resource_version (workspace_id, path, id DESC);

GRANT ALL ON resource_version TO windmill_admin;
GRANT ALL ON resource_version TO windmill_user;

ALTER TABLE resource_version ENABLE ROW LEVEL SECURITY;

CREATE POLICY admin_policy ON resource_version FOR ALL TO windmill_admin USING (true);

-- Visibility follows the parent resource: the subquery is itself subject to
-- resource's own path/extra_perms policies, so the several rules there stay
-- stated once instead of being mirrored (and left to drift) here.
CREATE POLICY see_parent_resource ON resource_version FOR ALL TO windmill_user
USING (
    EXISTS (
        SELECT 1 FROM resource r
        WHERE r.workspace_id = resource_version.workspace_id
          AND r.path = resource_version.path
    )
);

-- Seed the value every existing resource currently holds, so the first edit after
-- this migration has something to diff against. `state` and `cache` are machine
-- churn (setState, job result caching) and are excluded here and on write, the same
-- way workspace export excludes them.
INSERT INTO resource_version (workspace_id, path, resource_type, value, created_by, created_at)
SELECT workspace_id, path, resource_type, value, created_by, COALESCE(edited_at, now())
FROM resource
WHERE resource_type NOT IN ('state', 'cache');
