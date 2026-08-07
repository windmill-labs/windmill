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
-- churn (setState, job result caching) and are excluded here and in the trigger, the
-- same way workspace export excludes them (INTERNAL_RESOURCE_TYPES in
-- windmill-store/src/resources.rs — keep the two lists in step).
INSERT INTO resource_version (workspace_id, path, resource_type, value, created_by, created_at)
SELECT workspace_id, path, resource_type, value, created_by, COALESCE(edited_at, now())
FROM resource
WHERE resource_type NOT IN ('state', 'cache');

-- Recording lives in a trigger rather than the API handlers because resource values are
-- written from well outside them: a variable rename rewrites its linked resource
-- (windmill-store/src/variables.rs), workspace forks clone resources wholesale
-- (windmill-api-workspaces/src/workspaces.rs), native integrations upsert them
-- (windmill-native-triggers/src/workspace_integrations.rs), and trashbin restores
-- reinsert them (windmill-api/src/trash.rs). Handler-level hooks would miss all of those
-- and every path added later, leaving a history that silently disagrees with the value it
-- claims to describe.
CREATE OR REPLACE FUNCTION record_resource_version() RETURNS trigger AS $$
BEGIN
    IF NEW.resource_type IN ('state', 'cache') THEN
        RETURN NEW;
    END IF;

    -- A rename or a description edit leaves the value alone and so records nothing. This
    -- is also what keeps a trashbin restore from appending a duplicate of what it restored.
    IF TG_OP = 'UPDATE' AND NEW.value IS NOT DISTINCT FROM OLD.value THEN
        RETURN NEW;
    END IF;

    -- `session.user` is set by UserDB::begin for authed requests and absent for worker and
    -- system writes, which fall back to the row's own author.
    INSERT INTO resource_version (workspace_id, path, resource_type, value, created_by)
    VALUES (
        NEW.workspace_id, NEW.path, NEW.resource_type, NEW.value,
        COALESCE(current_setting('session.user', true), NEW.created_by)
    );

    -- Resources are not only edited by humans: `setResource` from a script reaches the same
    -- write path, so a scheduled job can grow one path's history without bound.
    DELETE FROM resource_version
    WHERE workspace_id = NEW.workspace_id AND path = NEW.path
      AND id < (
          SELECT min(id) FROM (
              SELECT id FROM resource_version
              WHERE workspace_id = NEW.workspace_id AND path = NEW.path
              ORDER BY id DESC LIMIT 100
          ) kept
      );

    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER record_resource_version_trigger
AFTER INSERT OR UPDATE ON resource
FOR EACH ROW EXECUTE FUNCTION record_resource_version();
