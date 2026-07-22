-- Add up migration script here
CREATE TABLE amqp_trigger (
    amqp_resource_path VARCHAR(255) NOT NULL,
    queue_name VARCHAR(255) NOT NULL,
    exchange JSONB NULL,
    options JSONB NULL,
    path VARCHAR(255) NOT NULL,
    script_path VARCHAR(255) NOT NULL,
    is_flow BOOLEAN NOT NULL,
    workspace_id VARCHAR(50) NOT NULL,
    edited_by VARCHAR(50) NOT NULL,
    edited_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    extra_perms JSONB NOT NULL DEFAULT '{}',
    server_id VARCHAR(50) NULL,
    last_server_ping TIMESTAMPTZ NULL,
    error TEXT NULL,
    error_handler_path VARCHAR(255) NULL,
    error_handler_args JSONB NULL,
    retry JSONB NULL,
    mode TRIGGER_MODE NOT NULL DEFAULT 'enabled'::TRIGGER_MODE,
    permissioned_as VARCHAR(255) NOT NULL,
    labels TEXT[] NULL,
    PRIMARY KEY (path, workspace_id),
    FOREIGN KEY (workspace_id) REFERENCES workspace(id) ON DELETE CASCADE
);

CREATE INDEX idx_amqp_trigger_labels ON amqp_trigger USING gin (labels) WHERE labels IS NOT NULL;

GRANT ALL ON amqp_trigger TO windmill_user;
GRANT ALL ON amqp_trigger TO windmill_admin;

ALTER TABLE amqp_trigger ENABLE ROW LEVEL SECURITY;

CREATE POLICY admin_policy ON amqp_trigger FOR ALL TO windmill_admin USING (true);

CREATE POLICY see_folder_extra_perms_user_select ON amqp_trigger FOR SELECT TO windmill_user
USING (SPLIT_PART(amqp_trigger.path, '/', 1) = 'f' AND SPLIT_PART(amqp_trigger.path, '/', 2) = any((select regexp_split_to_array(current_setting('session.folders_read'), ','))::text[]));
CREATE POLICY see_folder_extra_perms_user_insert ON amqp_trigger FOR INSERT TO windmill_user
WITH CHECK (SPLIT_PART(amqp_trigger.path, '/', 1) = 'f' AND SPLIT_PART(amqp_trigger.path, '/', 2) = any((select regexp_split_to_array(current_setting('session.folders_write'), ','))::text[]));
CREATE POLICY see_folder_extra_perms_user_update ON amqp_trigger FOR UPDATE TO windmill_user
USING (SPLIT_PART(amqp_trigger.path, '/', 1) = 'f' AND SPLIT_PART(amqp_trigger.path, '/', 2) = any((select regexp_split_to_array(current_setting('session.folders_write'), ','))::text[]));
CREATE POLICY see_folder_extra_perms_user_delete ON amqp_trigger FOR DELETE TO windmill_user
USING (SPLIT_PART(amqp_trigger.path, '/', 1) = 'f' AND SPLIT_PART(amqp_trigger.path, '/', 2) = any((select regexp_split_to_array(current_setting('session.folders_write'), ','))::text[]));

CREATE POLICY see_own ON amqp_trigger FOR ALL TO windmill_user
USING (SPLIT_PART(amqp_trigger.path, '/', 1) = 'u' AND SPLIT_PART(amqp_trigger.path, '/', 2) = (select current_setting('session.user')));
CREATE POLICY see_member ON amqp_trigger FOR ALL TO windmill_user
USING (SPLIT_PART(amqp_trigger.path, '/', 1) = 'g' AND SPLIT_PART(amqp_trigger.path, '/', 2) = any((select regexp_split_to_array(current_setting('session.groups'), ','))::text[]));

CREATE POLICY see_extra_perms_user_select ON amqp_trigger FOR SELECT TO windmill_user
USING (extra_perms ? (select concat('u/', current_setting('session.user'))));
CREATE POLICY see_extra_perms_user_insert ON amqp_trigger FOR INSERT TO windmill_user
WITH CHECK ((extra_perms ->> (select concat('u/', current_setting('session.user'))))::boolean);
CREATE POLICY see_extra_perms_user_update ON amqp_trigger FOR UPDATE TO windmill_user
USING ((extra_perms ->> (select concat('u/', current_setting('session.user'))))::boolean);
CREATE POLICY see_extra_perms_user_delete ON amqp_trigger FOR DELETE TO windmill_user
USING ((extra_perms ->> (select concat('u/', current_setting('session.user'))))::boolean);

CREATE POLICY see_extra_perms_groups_select ON amqp_trigger FOR SELECT TO windmill_user
USING (extra_perms ?| (select regexp_split_to_array(current_setting('session.pgroups'), ','))::text[]);
CREATE POLICY see_extra_perms_groups_insert ON amqp_trigger FOR INSERT TO windmill_user
WITH CHECK (exists(
    SELECT key, value FROM jsonb_each_text(extra_perms)
    WHERE SPLIT_PART(key, '/', 1) = 'g' AND key = ANY((select regexp_split_to_array(current_setting('session.pgroups'), ','))::text[])
    AND value::boolean));
CREATE POLICY see_extra_perms_groups_update ON amqp_trigger FOR UPDATE TO windmill_user
USING (exists(
    SELECT key, value FROM jsonb_each_text(extra_perms)
    WHERE SPLIT_PART(key, '/', 1) = 'g' AND key = ANY((select regexp_split_to_array(current_setting('session.pgroups'), ','))::text[])
    AND value::boolean));
CREATE POLICY see_extra_perms_groups_delete ON amqp_trigger FOR DELETE  TO windmill_user
USING (exists(
    SELECT key, value FROM jsonb_each_text(extra_perms)
    WHERE SPLIT_PART(key, '/', 1) = 'g' AND key = ANY((select regexp_split_to_array(current_setting('session.pgroups'), ','))::text[])
    AND value::boolean));

-- Enum values for the new trigger kind. ALTER TYPE ... ADD VALUE runs inside the
-- migration transaction on PG >= 14 (Windmill's minimum) as long as the value
-- isn't used in the same transaction — the amqp_trigger table above does not
-- reference these enum types.
ALTER TYPE TRIGGER_KIND ADD VALUE IF NOT EXISTS 'amqp';
ALTER TYPE job_trigger_kind ADD VALUE IF NOT EXISTS 'amqp';
ALTER TYPE draft_kind ADD VALUE IF NOT EXISTS 'trigger_amqp';
