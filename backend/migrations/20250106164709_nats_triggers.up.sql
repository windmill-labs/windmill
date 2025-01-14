ALTER TYPE trigger_kind ADD VALUE IF NOT EXISTS 'nats';

CREATE TABLE nats_trigger (
    path VARCHAR(255) NOT NULL,
    nats_resource_path VARCHAR(255) NOT NULL,
    subjects VARCHAR(255)[] NOT NULL,
    stream_name VARCHAR(255) NULL,
    consumer_name VARCHAR(255) NULL,
    use_jetstream BOOLEAN NOT NULL,
    script_path VARCHAR(255) NOT NULL,
    is_flow BOOLEAN NOT NULL,
    workspace_id VARCHAR(50) NOT NULL,
    edited_by VARCHAR(50) NOT NULL,
    email VARCHAR(255) NOT NULL,
    edited_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    extra_perms JSONB NOT NULL DEFAULT '{}',
    server_id VARCHAR(50) NULL,
    last_server_ping TIMESTAMPTZ NULL,
    error TEXT NULL,
    enabled BOOLEAN NOT NULL,
    PRIMARY KEY (path, workspace_id),
    FOREIGN KEY (workspace_id) REFERENCES workspace(id) ON DELETE CASCADE
);

GRANT ALL ON nats_trigger TO windmill_user;
GRANT ALL ON nats_trigger TO windmill_admin;

ALTER TABLE nats_trigger ENABLE ROW LEVEL SECURITY;

CREATE POLICY admin_policy ON nats_trigger FOR ALL TO windmill_admin USING (true);

CREATE POLICY see_folder_extra_perms_user_select ON nats_trigger FOR SELECT TO windmill_user
USING (SPLIT_PART(nats_trigger.path, '/', 1) = 'f' AND SPLIT_PART(nats_trigger.path, '/', 2) = any(regexp_split_to_array(current_setting('session.folders_read'), ',')::text[]));
CREATE POLICY see_folder_extra_perms_user_insert ON nats_trigger FOR INSERT TO windmill_user
WITH CHECK (SPLIT_PART(nats_trigger.path, '/', 1) = 'f' AND SPLIT_PART(nats_trigger.path, '/', 2) = any(regexp_split_to_array(current_setting('session.folders_write'), ',')::text[]));
CREATE POLICY see_folder_extra_perms_user_update ON nats_trigger FOR UPDATE TO windmill_user
USING (SPLIT_PART(nats_trigger.path, '/', 1) = 'f' AND SPLIT_PART(nats_trigger.path, '/', 2) = any(regexp_split_to_array(current_setting('session.folders_write'), ',')::text[]));
CREATE POLICY see_folder_extra_perms_user_delete ON nats_trigger FOR DELETE TO windmill_user
USING (SPLIT_PART(nats_trigger.path, '/', 1) = 'f' AND SPLIT_PART(nats_trigger.path, '/', 2) = any(regexp_split_to_array(current_setting('session.folders_write'), ',')::text[]));

CREATE POLICY see_own ON nats_trigger FOR ALL TO windmill_user
USING (SPLIT_PART(nats_trigger.path, '/', 1) = 'u' AND SPLIT_PART(nats_trigger.path, '/', 2) = current_setting('session.user'));
CREATE POLICY see_member ON nats_trigger FOR ALL TO windmill_user
USING (SPLIT_PART(nats_trigger.path, '/', 1) = 'g' AND SPLIT_PART(nats_trigger.path, '/', 2) = any(regexp_split_to_array(current_setting('session.groups'), ',')::text[]));

CREATE POLICY see_extra_perms_user_select ON nats_trigger FOR SELECT TO windmill_user
USING (extra_perms ? CONCAT('u/', current_setting('session.user')));
CREATE POLICY see_extra_perms_user_insert ON nats_trigger FOR INSERT TO windmill_user
WITH CHECK ((extra_perms ->> CONCAT('u/', current_setting('session.user')))::boolean);
CREATE POLICY see_extra_perms_user_update ON nats_trigger FOR UPDATE TO windmill_user
USING ((extra_perms ->> CONCAT('u/', current_setting('session.user')))::boolean);
CREATE POLICY see_extra_perms_user_delete ON nats_trigger FOR DELETE TO windmill_user
USING ((extra_perms ->> CONCAT('u/', current_setting('session.user')))::boolean);

CREATE POLICY see_extra_perms_groups_select ON nats_trigger FOR SELECT TO windmill_user
USING (extra_perms ?| regexp_split_to_array(current_setting('session.pgroups'), ',')::text[]);
CREATE POLICY see_extra_perms_groups_insert ON nats_trigger FOR INSERT TO windmill_user
WITH CHECK (exists(
    SELECT key, value FROM jsonb_each_text(extra_perms) 
    WHERE SPLIT_PART(key, '/', 1) = 'g' AND key = ANY(regexp_split_to_array(current_setting('session.pgroups'), ',')::text[])
    AND value::boolean));
CREATE POLICY see_extra_perms_groups_update ON nats_trigger FOR UPDATE TO windmill_user
USING (exists(
    SELECT key, value FROM jsonb_each_text(extra_perms) 
    WHERE SPLIT_PART(key, '/', 1) = 'g' AND key = ANY(regexp_split_to_array(current_setting('session.pgroups'), ',')::text[])
    AND value::boolean));
CREATE POLICY see_extra_perms_groups_delete ON nats_trigger FOR DELETE  TO windmill_user
USING (exists(
    SELECT key, value FROM jsonb_each_text(extra_perms) 
    WHERE SPLIT_PART(key, '/', 1) = 'g' AND key = ANY(regexp_split_to_array(current_setting('session.pgroups'), ',')::text[])
    AND value::boolean));