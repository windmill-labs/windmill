-- Add up migration script here

-- Folder permission changes history
CREATE TABLE IF NOT EXISTS folder_permission_history (
    id BIGSERIAL PRIMARY KEY,
    workspace_id VARCHAR(50) NOT NULL,
    folder_name VARCHAR(255) NOT NULL,
    changed_by VARCHAR(50) NOT NULL,
    changed_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    change_type VARCHAR(50) NOT NULL,
    affected VARCHAR(100),
    FOREIGN KEY (workspace_id, folder_name) REFERENCES folder(workspace_id, name) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_folder_perm_history_workspace_folder
    ON folder_permission_history(workspace_id, folder_name, id DESC);

-- Group permission changes history
CREATE TABLE IF NOT EXISTS group_permission_history (
    id BIGSERIAL PRIMARY KEY,
    workspace_id VARCHAR(50) NOT NULL,
    group_name VARCHAR(255) NOT NULL,
    changed_by VARCHAR(50) NOT NULL,
    changed_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    change_type VARCHAR(50) NOT NULL,
    member_affected VARCHAR(100),
    FOREIGN KEY (workspace_id, group_name) REFERENCES group_(workspace_id, name) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_group_perm_history_workspace_group
    ON group_permission_history(workspace_id, group_name, id DESC);

GRANT ALL ON TABLE folder_permission_history TO windmill_user;
GRANT ALL ON TABLE group_permission_history TO windmill_user;
GRANT ALL ON TABLE folder_permission_history TO windmill_admin;
GRANT ALL ON TABLE group_permission_history TO windmill_admin;

-- Enable RLS on folder_permission_history
ALTER TABLE folder_permission_history ENABLE ROW LEVEL SECURITY;

-- Admin policies (windmill_admin can always do everything)
CREATE POLICY admin_all ON folder_permission_history FOR ALL TO windmill_admin USING (true) WITH CHECK (true);
CREATE POLICY admin_all ON group_permission_history FOR ALL TO windmill_admin USING (true) WITH CHECK (true);

-- Enable RLS on group_permission_history
ALTER TABLE group_permission_history ENABLE ROW LEVEL SECURITY;

-- RLS policies for folder_permission_history
-- Anyone can insert
CREATE POLICY allow_insert ON folder_permission_history FOR INSERT TO windmill_user WITH CHECK (true);

-- Select requires being in extra_perms (as user or via group)
CREATE POLICY see_extra_perms_user ON folder_permission_history FOR SELECT TO windmill_user
USING (EXISTS (
    SELECT 1 FROM folder f
    WHERE f.workspace_id = folder_permission_history.workspace_id
    AND f.name = folder_permission_history.folder_name
    AND f.extra_perms ? CONCAT('u/', current_setting('session.user'))
));

CREATE POLICY see_extra_perms_groups ON folder_permission_history FOR SELECT TO windmill_user
USING (EXISTS (
    SELECT 1 FROM folder f
    WHERE f.workspace_id = folder_permission_history.workspace_id
    AND f.name = folder_permission_history.folder_name
    AND f.extra_perms ?| regexp_split_to_array(current_setting('session.pgroups'), ',')::text[]
));

-- RLS policies for group_permission_history
-- Anyone can insert
CREATE POLICY allow_insert ON group_permission_history FOR INSERT TO windmill_user WITH CHECK (true);

-- Select requires being in extra_perms (as user or via group)
CREATE POLICY see_extra_perms_user ON group_permission_history FOR SELECT TO windmill_user
USING (EXISTS (
    SELECT 1 FROM group_ g
    WHERE g.workspace_id = group_permission_history.workspace_id
    AND g.name = group_permission_history.group_name
    AND (g.extra_perms ->> CONCAT('u/', current_setting('session.user')))::boolean
));

CREATE POLICY see_extra_perms_groups ON group_permission_history FOR SELECT TO windmill_user
USING (EXISTS (
    SELECT 1 FROM group_ g, jsonb_each_text(g.extra_perms) f
    WHERE g.workspace_id = group_permission_history.workspace_id
    AND g.name = group_permission_history.group_name
    AND SPLIT_PART(f.key, '/', 1) = 'g'
    AND f.key = ANY(regexp_split_to_array(current_setting('session.pgroups'), ',')::text[])
    AND f.value::boolean
));
