-- Add up migration script here

ALTER TYPE TRIGGER_KIND ADD VALUE IF NOT EXISTS 'azure';
ALTER TYPE JOB_TRIGGER_KIND ADD VALUE IF NOT EXISTS 'azure';

CREATE TYPE AZURE_TRIGGER_MODE AS ENUM ('basic_push', 'namespace_push', 'namespace_pull');

CREATE TABLE azure_trigger (
    azure_resource_path   VARCHAR(255) NOT NULL,
    azure_mode            AZURE_TRIGGER_MODE NOT NULL,
    scope_resource_id     TEXT NOT NULL,
    topic_name            VARCHAR(255),
    subscription_name     VARCHAR(255) NOT NULL,
    event_type_filters    JSONB,
    push_auth_config      JSONB,
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
    mode                  TRIGGER_MODE NOT NULL DEFAULT 'enabled',
    permissioned_as       VARCHAR(255) NOT NULL,
    error_handler_path    VARCHAR(255),
    error_handler_args    JSONB,
    retry                 JSONB,
    labels                TEXT[],
    PRIMARY KEY (path, workspace_id),
    CONSTRAINT azure_topic_name_matches_mode CHECK (
        (azure_mode = 'basic_push' AND topic_name IS NULL) OR
        (azure_mode IN ('namespace_push', 'namespace_pull') AND topic_name IS NOT NULL)
    ),
    CONSTRAINT azure_push_auth_config_matches_mode CHECK (
        (azure_mode IN ('basic_push', 'namespace_push') AND push_auth_config IS NOT NULL) OR
        (azure_mode = 'namespace_pull' AND push_auth_config IS NULL)
    )
);

CREATE UNIQUE INDEX unique_subscription_per_azure_scope
ON azure_trigger (subscription_name, scope_resource_id, workspace_id);

CREATE INDEX idx_azure_trigger_labels ON azure_trigger USING GIN (labels) WHERE labels IS NOT NULL;

GRANT ALL ON azure_trigger TO windmill_user;
GRANT ALL ON azure_trigger TO windmill_admin;

ALTER TABLE azure_trigger ENABLE ROW LEVEL SECURITY;

CREATE POLICY admin_policy ON azure_trigger FOR ALL TO windmill_admin USING (true);

CREATE POLICY see_folder_extra_perms_user_select ON azure_trigger FOR SELECT TO windmill_user
USING (SPLIT_PART(azure_trigger.path, '/', 1) = 'f' AND SPLIT_PART(azure_trigger.path, '/', 2) = any(regexp_split_to_array(current_setting('session.folders_read'), ',')::text[]));
CREATE POLICY see_folder_extra_perms_user_insert ON azure_trigger FOR INSERT TO windmill_user
WITH CHECK (SPLIT_PART(azure_trigger.path, '/', 1) = 'f' AND SPLIT_PART(azure_trigger.path, '/', 2) = any(regexp_split_to_array(current_setting('session.folders_write'), ',')::text[]));
CREATE POLICY see_folder_extra_perms_user_update ON azure_trigger FOR UPDATE TO windmill_user
USING (SPLIT_PART(azure_trigger.path, '/', 1) = 'f' AND SPLIT_PART(azure_trigger.path, '/', 2) = any(regexp_split_to_array(current_setting('session.folders_write'), ',')::text[]));
CREATE POLICY see_folder_extra_perms_user_delete ON azure_trigger FOR DELETE TO windmill_user
USING (SPLIT_PART(azure_trigger.path, '/', 1) = 'f' AND SPLIT_PART(azure_trigger.path, '/', 2) = any(regexp_split_to_array(current_setting('session.folders_write'), ',')::text[]));

CREATE POLICY see_own ON azure_trigger FOR ALL TO windmill_user
USING (SPLIT_PART(azure_trigger.path, '/', 1) = 'u' AND SPLIT_PART(azure_trigger.path, '/', 2) = current_setting('session.user'));
CREATE POLICY see_member ON azure_trigger FOR ALL TO windmill_user
USING (SPLIT_PART(azure_trigger.path, '/', 1) = 'g' AND SPLIT_PART(azure_trigger.path, '/', 2) = any(regexp_split_to_array(current_setting('session.groups'), ',')::text[]));

CREATE POLICY see_extra_perms_user_select ON azure_trigger FOR SELECT TO windmill_user
USING (extra_perms ? CONCAT('u/', current_setting('session.user')));
CREATE POLICY see_extra_perms_user_insert ON azure_trigger FOR INSERT TO windmill_user
WITH CHECK ((extra_perms ->> CONCAT('u/', current_setting('session.user')))::boolean);
CREATE POLICY see_extra_perms_user_update ON azure_trigger FOR UPDATE TO windmill_user
USING ((extra_perms ->> CONCAT('u/', current_setting('session.user')))::boolean);
CREATE POLICY see_extra_perms_user_delete ON azure_trigger FOR DELETE TO windmill_user
USING ((extra_perms ->> CONCAT('u/', current_setting('session.user')))::boolean);

CREATE POLICY see_extra_perms_groups_select ON azure_trigger FOR SELECT TO windmill_user
USING (extra_perms ?| regexp_split_to_array(current_setting('session.pgroups'), ',')::text[]);
CREATE POLICY see_extra_perms_groups_insert ON azure_trigger FOR INSERT TO windmill_user
WITH CHECK (exists(
    SELECT key, value FROM jsonb_each_text(extra_perms)
    WHERE SPLIT_PART(key, '/', 1) = 'g' AND key = ANY(regexp_split_to_array(current_setting('session.pgroups'), ',')::text[])
    AND value::boolean));
CREATE POLICY see_extra_perms_groups_update ON azure_trigger FOR UPDATE TO windmill_user
USING (exists(
    SELECT key, value FROM jsonb_each_text(extra_perms)
    WHERE SPLIT_PART(key, '/', 1) = 'g' AND key = ANY(regexp_split_to_array(current_setting('session.pgroups'), ',')::text[])
    AND value::boolean));
CREATE POLICY see_extra_perms_groups_delete ON azure_trigger FOR DELETE TO windmill_user
USING (exists(
    SELECT key, value FROM jsonb_each_text(extra_perms)
    WHERE SPLIT_PART(key, '/', 1) = 'g' AND key = ANY(regexp_split_to_array(current_setting('session.pgroups'), ',')::text[])
    AND value::boolean));
