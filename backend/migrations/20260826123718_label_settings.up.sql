-- Per-label presentation, sparse: a label only gets a row once someone picks a
-- colour for it. Labels themselves stay what they have always been — bare
-- strings in the `labels text[]` columns — so a label with no row here is still
-- a perfectly ordinary label. `name` is TEXT rather than VARCHAR(50) because the
-- item-side arrays impose no length limit; capping it here would make this table
-- the constraint on what a label may be called.
CREATE TABLE IF NOT EXISTS label_settings (
    workspace_id VARCHAR(50) NOT NULL REFERENCES workspace(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    color VARCHAR(20) NOT NULL,
    PRIMARY KEY (workspace_id, name)
);

GRANT ALL ON TABLE label_settings TO windmill_user;
GRANT ALL ON TABLE label_settings TO windmill_admin;

ALTER TABLE label_settings ENABLE ROW LEVEL SECURITY;

CREATE POLICY admin_all ON label_settings FOR ALL TO windmill_admin USING (true) WITH CHECK (true);

-- No per-row restriction for windmill_user: a colour carries no more information
-- than the label name it decorates, and the whole label vocabulary of a workspace
-- is already readable by any member through labels/list. Workspace isolation
-- comes from `workspace_id = $1` in every query, the same way labels/list is
-- scoped. Operators are turned away at the handler, not here — RLS cannot tell
-- an operator from any other windmill_user.
CREATE POLICY all_members ON label_settings FOR ALL TO windmill_user USING (true) WITH CHECK (true);
