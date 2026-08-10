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

-- Read visibility follows the parent resource: the subquery is itself subject to resource's
-- own path/extra_perms policies, so the several rules there stay stated once instead of being
-- mirrored (and left to drift) here.
--
-- SELECT only, deliberately. `FOR ALL ... USING` would be reused as the INSERT/UPDATE/DELETE
-- check expression, and since the subquery is a SELECT it applies resource's *read* policies —
-- which would let a user with read-only access to a resource write its history. Nothing but the
-- trigger below writes this table, and it is SECURITY DEFINER so it does not need a policy.
CREATE POLICY see_parent_resource ON resource_version FOR SELECT TO windmill_user
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
--
-- Deleting a resource hard-deletes it, so the FK cascade takes its whole history with it: a
-- trashbin restore lands on an empty history and starts a new one at v1.
CREATE OR REPLACE FUNCTION record_resource_version() RETURNS trigger AS $$
BEGIN
    -- `session.user` is set by UserDB::begin for authed requests; worker and system writes fall
    -- back to the row's own author. NULLIF because a transaction-local set_config resets the
    -- placeholder to the empty string rather than unsetting it, so a pooled connection that
    -- previously served an authed request reports '' here, not NULL.
    INSERT INTO resource_version (workspace_id, path, resource_type, value, created_by)
    VALUES (
        NEW.workspace_id, NEW.path, NEW.resource_type, NEW.value,
        COALESCE(NULLIF(current_setting('session.user', true), ''), NEW.created_by)
    );

    -- The per-path cap is enforced by trim_resource_versions in the monitor, not here: trimming
    -- on every write would tax a path `setResource` can drive in a loop, to keep a bound that
    -- does not need to hold instantaneously.

    RETURN NEW;
END;
-- SECURITY DEFINER so history is written on behalf of every writer, including the read-only
-- policy above, without granting anyone direct write access to the table. `SET search_path FROM
-- CURRENT` is the injection hardening that goes with it, captured rather than hardcoded so
-- installs running a non-public PG_SCHEMA still resolve (see
-- 20260624103600_repair_folder_labels_search_path.up.sql).
$$ LANGUAGE plpgsql SECURITY DEFINER SET search_path FROM CURRENT;

-- Split in two, and gated in WHEN rather than in the function body, so the rows that never
-- produce a version never enter plpgsql at all: `state`/`cache` writes (one per setState and
-- per cached job result — by far the hottest writers of this table) and updates that leave the
-- value alone. Separate triggers because a WHEN on INSERT cannot reference OLD.
CREATE TRIGGER record_resource_version_insert_trigger
AFTER INSERT ON resource
FOR EACH ROW
WHEN (NEW.resource_type NOT IN ('state', 'cache'))
EXECUTE FUNCTION record_resource_version();

CREATE TRIGGER record_resource_version_update_trigger
AFTER UPDATE ON resource
FOR EACH ROW
WHEN (
    NEW.resource_type NOT IN ('state', 'cache')
    AND NEW.value IS DISTINCT FROM OLD.value
)
EXECUTE FUNCTION record_resource_version();
