-- Add up migration script here

ALTER TYPE TRIGGER_KIND ADD VALUE IF NOT EXISTS 'gcp';
CREATE TYPE DELIVERY_MODE AS ENUM ('push', 'pull');

CREATE TABLE gcp_trigger (
    gcp_resource_path     VARCHAR(255) NOT NULL,
    topic_id              VARCHAR(255) NOT NULL CHECK (
                              CHAR_LENGTH(topic_id) BETWEEN 3 AND 255
                           ),
    subscription_id       VARCHAR(255) NOT NULL CHECK (
                              CHAR_LENGTH(subscription_id) BETWEEN 3 AND 255
                           ),
    delivery_type         DELIVERY_MODE NOT NULL,
    delivery_config       JSONB NULL CHECK (delivery_type != 'push'::DELIVERY_MODE OR (delivery_config IS NOT NULL)),
    path                  VARCHAR(255) NOT NULL,
    script_path           VARCHAR(255) NOT NULL,
    is_flow               BOOLEAN NOT NULL,
    workspace_id          VARCHAR(50) NOT NULL,
    edited_by             VARCHAR(50) NOT NULL,
    email                 VARCHAR(255) NOT NULL,
    edited_at             TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    extra_perms           JSONB NOT NULL DEFAULT '{}',
    server_id             VARCHAR(50),
    last_server_ping      TIMESTAMPTZ,
    error                 TEXT,
    enabled               BOOLEAN NOT NULL,
    PRIMARY KEY (path, workspace_id)
);

CREATE UNIQUE INDEX unique_subscription_per_gcp_resource
ON gcp_trigger (subscription_id, gcp_resource_path, workspace_id);

GRANT ALL ON gcp_trigger TO windmill_user;
GRANT ALL ON gcp_trigger TO windmill_admin;

ALTER TABLE gcp_trigger ENABLE ROW LEVEL SECURITY;

CREATE POLICY admin_policy ON gcp_trigger FOR ALL TO windmill_admin USING (true);

CREATE POLICY see_folder_extra_perms_user_select ON gcp_trigger FOR SELECT TO windmill_user
USING (SPLIT_PART(gcp_trigger.path, '/', 1) = 'f' AND SPLIT_PART(gcp_trigger.path, '/', 2) = any(regexp_split_to_array(current_setting('session.folders_read'), ',')::text[]));
CREATE POLICY see_folder_extra_perms_user_insert ON gcp_trigger FOR INSERT TO windmill_user
WITH CHECK (SPLIT_PART(gcp_trigger.path, '/', 1) = 'f' AND SPLIT_PART(gcp_trigger.path, '/', 2) = any(regexp_split_to_array(current_setting('session.folders_write'), ',')::text[]));
CREATE POLICY see_folder_extra_perms_user_update ON gcp_trigger FOR UPDATE TO windmill_user
USING (SPLIT_PART(gcp_trigger.path, '/', 1) = 'f' AND SPLIT_PART(gcp_trigger.path, '/', 2) = any(regexp_split_to_array(current_setting('session.folders_write'), ',')::text[]));
CREATE POLICY see_folder_extra_perms_user_delete ON gcp_trigger FOR DELETE TO windmill_user
USING (SPLIT_PART(gcp_trigger.path, '/', 1) = 'f' AND SPLIT_PART(gcp_trigger.path, '/', 2) = any(regexp_split_to_array(current_setting('session.folders_write'), ',')::text[]));

CREATE POLICY see_own ON gcp_trigger FOR ALL TO windmill_user
USING (SPLIT_PART(gcp_trigger.path, '/', 1) = 'u' AND SPLIT_PART(gcp_trigger.path, '/', 2) = current_setting('session.user'));
CREATE POLICY see_member ON gcp_trigger FOR ALL TO windmill_user
USING (SPLIT_PART(gcp_trigger.path, '/', 1) = 'g' AND SPLIT_PART(gcp_trigger.path, '/', 2) = any(regexp_split_to_array(current_setting('session.groups'), ',')::text[]));

CREATE POLICY see_extra_perms_user_select ON gcp_trigger FOR SELECT TO windmill_user
USING (extra_perms ? CONCAT('u/', current_setting('session.user')));
CREATE POLICY see_extra_perms_user_insert ON gcp_trigger FOR INSERT TO windmill_user
WITH CHECK ((extra_perms ->> CONCAT('u/', current_setting('session.user')))::boolean);
CREATE POLICY see_extra_perms_user_update ON gcp_trigger FOR UPDATE TO windmill_user
USING ((extra_perms ->> CONCAT('u/', current_setting('session.user')))::boolean);
CREATE POLICY see_extra_perms_user_delete ON gcp_trigger FOR DELETE TO windmill_user
USING ((extra_perms ->> CONCAT('u/', current_setting('session.user')))::boolean);

CREATE POLICY see_extra_perms_groups_select ON gcp_trigger FOR SELECT TO windmill_user
USING (extra_perms ?| regexp_split_to_array(current_setting('session.pgroups'), ',')::text[]);
CREATE POLICY see_extra_perms_groups_insert ON gcp_trigger FOR INSERT TO windmill_user
WITH CHECK (exists(
    SELECT key, value FROM jsonb_each_text(extra_perms) 
    WHERE SPLIT_PART(key, '/', 1) = 'g' AND key = ANY(regexp_split_to_array(current_setting('session.pgroups'), ',')::text[])
    AND value::boolean));
CREATE POLICY see_extra_perms_groups_update ON gcp_trigger FOR UPDATE TO windmill_user
USING (exists(
    SELECT key, value FROM jsonb_each_text(extra_perms) 
    WHERE SPLIT_PART(key, '/', 1) = 'g' AND key = ANY(regexp_split_to_array(current_setting('session.pgroups'), ',')::text[])
    AND value::boolean));
CREATE POLICY see_extra_perms_groups_delete ON gcp_trigger FOR DELETE  TO windmill_user
USING (exists(
    SELECT key, value FROM jsonb_each_text(extra_perms) 
    WHERE SPLIT_PART(key, '/', 1) = 'g' AND key = ANY(regexp_split_to_array(current_setting('session.pgroups'), ',')::text[])
    AND value::boolean));