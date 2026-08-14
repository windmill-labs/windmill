-- Append-only record of every schedule/trigger mutation: who, what changed, and
-- from which kind of client.
CREATE TABLE IF NOT EXISTS trigger_history (
    id BIGSERIAL PRIMARY KEY,
    workspace_id VARCHAR(50) NOT NULL REFERENCES workspace(id) ON DELETE CASCADE,
    -- 'schedule' or a trigger's TRIGGER_TYPE ('http', 'kafka', ...). Not the
    -- TRIGGER_KIND enum: that one is capture-oriented and misses 'schedule'.
    trigger_kind VARCHAR(50) NOT NULL,
    path VARCHAR(255) NOT NULL,
    -- 'create' | 'update' | 'delete' | 'enable' | 'disable' | 'suspend'
    operation VARCHAR(20) NOT NULL,
    -- 'ui' | 'cli' | 'api' | 'worker'
    source VARCHAR(20) NOT NULL,
    -- NULL when the server acted on its own (worker auto-disable).
    username VARCHAR(255),
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    -- {field: {old, new}} for the fields that actually changed. `old` is
    -- absent where it is not known: a create, and the workspace-wide handler
    -- override that rewrites every schedule without reading them first. NULL
    -- when the operation carries no field-level diff at all (delete).
    changes JSONB
);

CREATE INDEX IF NOT EXISTS idx_trigger_history_workspace_kind_path
    ON trigger_history(workspace_id, trigger_kind, path, id DESC);

CREATE INDEX IF NOT EXISTS idx_trigger_history_workspace_id
    ON trigger_history(workspace_id, id DESC);

GRANT ALL ON TABLE trigger_history TO windmill_user;
GRANT ALL ON TABLE trigger_history TO windmill_admin;
GRANT ALL ON SEQUENCE trigger_history_id_seq TO windmill_user;
GRANT ALL ON SEQUENCE trigger_history_id_seq TO windmill_admin;

ALTER TABLE trigger_history ENABLE ROW LEVEL SECURITY;

CREATE POLICY admin_all ON trigger_history FOR ALL TO windmill_admin USING (true) WITH CHECK (true);

-- Every mutating trigger route writes through the RLS pool, so windmill_user
-- must be able to append.
CREATE POLICY allow_insert ON trigger_history FOR INSERT TO windmill_user WITH CHECK (true);

-- Reads mirror the path half of the live trigger's own policies: a row can
-- quote a schedule's `args`, so it must not be readable by anyone who could not
-- read the trigger it describes. Deliberately narrower than the live row on one
-- point — the `extra_perms` grants have no counterpart here, since the history
-- does not carry the row's ACL and must survive its deletion. Narrower is the
-- safe direction.
CREATE POLICY see_own ON trigger_history FOR SELECT TO windmill_user
USING (
    SPLIT_PART(path::text, '/', 1) = 'u'
    AND SPLIT_PART(path::text, '/', 2) = current_setting('session.user')
);

CREATE POLICY see_member ON trigger_history FOR SELECT TO windmill_user
USING (
    SPLIT_PART(path::text, '/', 1) = 'g'
    AND SPLIT_PART(path::text, '/', 2) = ANY(regexp_split_to_array(current_setting('session.groups'), ','))
);

CREATE POLICY see_folder_extra_perms_user ON trigger_history FOR SELECT TO windmill_user
USING (
    SPLIT_PART(path::text, '/', 1) = 'f'
    AND SPLIT_PART(path::text, '/', 2) = ANY(regexp_split_to_array(current_setting('session.folders_read'), ','))
);
