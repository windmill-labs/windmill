-- `asset_trigger` holds one row per reactive asset-change subscription.
-- Implicit triggers (is_implicit=true) are projected from `#trigger: asset`
-- annotations on every script save. Never editable via API/CLI.

CREATE TABLE asset_trigger (
    workspace_id VARCHAR(50) NOT NULL REFERENCES workspace(id) ON DELETE CASCADE ON UPDATE CASCADE,
    path VARCHAR(255) NOT NULL,
    script_path VARCHAR(255) NOT NULL,
    is_flow BOOLEAN NOT NULL DEFAULT false,

    -- implicit-trigger pinning
    owner_script_path VARCHAR(255),
    owner_script_hash BIGINT,
    is_implicit BOOLEAN NOT NULL DEFAULT false,

    -- trigger config
    on_event VARCHAR(32) NOT NULL,
    subscription_set JSONB NOT NULL,
    fires VARCHAR(16) NOT NULL DEFAULT 'all',
    debounce_s INT NOT NULL DEFAULT 30,
    partition_map JSONB,
    cancel_on_new BOOLEAN NOT NULL DEFAULT false,
    backlog VARCHAR(16) NOT NULL DEFAULT 'coalesce',

    -- boilerplate shared with other trigger types
    error TEXT,
    server_id VARCHAR(64),
    last_server_ping TIMESTAMPTZ,
    error_handler_path VARCHAR(255),
    error_handler_args JSONB,
    retry JSONB,
    extra_perms JSONB NOT NULL DEFAULT '{}',
    edited_by VARCHAR(255) NOT NULL,
    edited_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    email VARCHAR(255) NOT NULL,
    mode TRIGGER_MODE NOT NULL DEFAULT 'enabled',
    labels TEXT[],

    PRIMARY KEY (workspace_id, path)
);

CREATE INDEX asset_trigger_owner_idx ON asset_trigger (workspace_id, owner_script_path) WHERE is_implicit = true;
CREATE INDEX asset_trigger_subscription_set_idx ON asset_trigger USING GIN (subscription_set);

GRANT ALL ON asset_trigger TO windmill_user;
GRANT ALL ON asset_trigger TO windmill_admin;

ALTER TABLE asset_trigger ENABLE ROW LEVEL SECURITY;

CREATE POLICY admin_policy ON asset_trigger FOR ALL TO windmill_admin USING (true);

CREATE POLICY see_folder_extra_perms_user_select ON asset_trigger FOR SELECT TO windmill_user
USING (SPLIT_PART(asset_trigger.path, '/', 1) = 'f' AND SPLIT_PART(asset_trigger.path, '/', 2) = any(regexp_split_to_array(current_setting('session.folders_read'), ',')::text[]));
CREATE POLICY see_folder_extra_perms_user_insert ON asset_trigger FOR INSERT TO windmill_user
WITH CHECK (SPLIT_PART(asset_trigger.path, '/', 1) = 'f' AND SPLIT_PART(asset_trigger.path, '/', 2) = any(regexp_split_to_array(current_setting('session.folders_write'), ',')::text[]));
CREATE POLICY see_folder_extra_perms_user_update ON asset_trigger FOR UPDATE TO windmill_user
USING (SPLIT_PART(asset_trigger.path, '/', 1) = 'f' AND SPLIT_PART(asset_trigger.path, '/', 2) = any(regexp_split_to_array(current_setting('session.folders_write'), ',')::text[]));
CREATE POLICY see_folder_extra_perms_user_delete ON asset_trigger FOR DELETE TO windmill_user
USING (SPLIT_PART(asset_trigger.path, '/', 1) = 'f' AND SPLIT_PART(asset_trigger.path, '/', 2) = any(regexp_split_to_array(current_setting('session.folders_write'), ',')::text[]));

CREATE POLICY see_own ON asset_trigger FOR ALL TO windmill_user
USING (SPLIT_PART(asset_trigger.path, '/', 1) = 'u' AND SPLIT_PART(asset_trigger.path, '/', 2) = current_setting('session.user'));
CREATE POLICY see_member ON asset_trigger FOR ALL TO windmill_user
USING (SPLIT_PART(asset_trigger.path, '/', 1) = 'g' AND SPLIT_PART(asset_trigger.path, '/', 2) = any(regexp_split_to_array(current_setting('session.groups'), ',')::text[]));

CREATE POLICY see_extra_perms_user_select ON asset_trigger FOR SELECT TO windmill_user
USING (extra_perms ? CONCAT('u/', current_setting('session.user')));
CREATE POLICY see_extra_perms_user_insert ON asset_trigger FOR INSERT TO windmill_user
WITH CHECK ((extra_perms ->> CONCAT('u/', current_setting('session.user')))::boolean);
CREATE POLICY see_extra_perms_user_update ON asset_trigger FOR UPDATE TO windmill_user
USING ((extra_perms ->> CONCAT('u/', current_setting('session.user')))::boolean);
CREATE POLICY see_extra_perms_user_delete ON asset_trigger FOR DELETE TO windmill_user
USING ((extra_perms ->> CONCAT('u/', current_setting('session.user')))::boolean);

CREATE POLICY see_extra_perms_groups_select ON asset_trigger FOR SELECT TO windmill_user
USING (extra_perms ?| regexp_split_to_array(current_setting('session.pgroups'), ',')::text[]);
CREATE POLICY see_extra_perms_groups_insert ON asset_trigger FOR INSERT TO windmill_user
WITH CHECK (exists(
    SELECT key, value FROM jsonb_each_text(extra_perms)
    WHERE SPLIT_PART(key, '/', 1) = 'g' AND key = ANY(regexp_split_to_array(current_setting('session.pgroups'), ',')::text[])
    AND value::boolean));
CREATE POLICY see_extra_perms_groups_update ON asset_trigger FOR UPDATE TO windmill_user
USING (exists(
    SELECT key, value FROM jsonb_each_text(extra_perms)
    WHERE SPLIT_PART(key, '/', 1) = 'g' AND key = ANY(regexp_split_to_array(current_setting('session.pgroups'), ',')::text[])
    AND value::boolean));
CREATE POLICY see_extra_perms_groups_delete ON asset_trigger FOR DELETE TO windmill_user
USING (exists(
    SELECT key, value FROM jsonb_each_text(extra_perms)
    WHERE SPLIT_PART(key, '/', 1) = 'g' AND key = ANY(regexp_split_to_array(current_setting('session.pgroups'), ',')::text[])
    AND value::boolean));
