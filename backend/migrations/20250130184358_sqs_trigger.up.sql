-- Add up migration script here
CREATE TABLE sqs_trigger(
    path VARCHAR(255) NOT NULL,
    queue_url VARCHAR(255) NOT NULL,
    aws_resource_path VARCHAR(255) NOT NULL,
    message_attributes TEXT[],
    script_path VARCHAR(255) NOT NULL,
    is_flow BOOLEAN NOT NULL,
    workspace_id VARCHAR(50) NOT NULL,
    edited_by VARCHAR(50) NOT NULL,
    email VARCHAR(255) NOT NULL,
    edited_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    extra_perms JSONB NULL,
    error TEXT NULL,
    server_id VARCHAR(50) NULL,
    last_server_ping TIMESTAMPTZ NULL,
    enabled BOOLEAN NOT NULL,
    CONSTRAINT PK_sqs_trigger PRIMARY KEY (path,workspace_id),
    CONSTRAINT fk_sqs_trigger_workspace FOREIGN KEY (workspace_id)
        REFERENCES workspace(id) ON DELETE CASCADE
);

GRANT ALL ON sqs_trigger TO windmill_user;
GRANT ALL ON sqs_trigger TO windmill_admin;

ALTER TABLE sqs_trigger ENABLE ROW LEVEL SECURITY;

CREATE POLICY admin_policy ON sqs_trigger FOR ALL TO windmill_admin USING (true);

CREATE POLICY see_folder_extra_perms_user_select ON sqs_trigger FOR SELECT TO windmill_user
USING (SPLIT_PART(sqs_trigger.path, '/', 1) = 'f' AND SPLIT_PART(sqs_trigger.path, '/', 2) = any(regexp_split_to_array(current_setting('session.folders_read'), ',')::text[]));
CREATE POLICY see_folder_extra_perms_user_insert ON sqs_trigger FOR INSERT TO windmill_user
WITH CHECK (SPLIT_PART(sqs_trigger.path, '/', 1) = 'f' AND SPLIT_PART(sqs_trigger.path, '/', 2) = any(regexp_split_to_array(current_setting('session.folders_write'), ',')::text[]));
CREATE POLICY see_folder_extra_perms_user_update ON sqs_trigger FOR UPDATE TO windmill_user
USING (SPLIT_PART(sqs_trigger.path, '/', 1) = 'f' AND SPLIT_PART(sqs_trigger.path, '/', 2) = any(regexp_split_to_array(current_setting('session.folders_write'), ',')::text[]));
CREATE POLICY see_folder_extra_perms_user_delete ON sqs_trigger FOR DELETE TO windmill_user
USING (SPLIT_PART(sqs_trigger.path, '/', 1) = 'f' AND SPLIT_PART(sqs_trigger.path, '/', 2) = any(regexp_split_to_array(current_setting('session.folders_write'), ',')::text[]));

CREATE POLICY see_own ON sqs_trigger FOR ALL TO windmill_user
USING (SPLIT_PART(sqs_trigger.path, '/', 1) = 'u' AND SPLIT_PART(sqs_trigger.path, '/', 2) = current_setting('session.user'));
CREATE POLICY see_member ON sqs_trigger FOR ALL TO windmill_user
USING (SPLIT_PART(sqs_trigger.path, '/', 1) = 'g' AND SPLIT_PART(sqs_trigger.path, '/', 2) = any(regexp_split_to_array(current_setting('session.groups'), ',')::text[]));

CREATE POLICY see_extra_perms_user_select ON sqs_trigger FOR SELECT TO windmill_user
USING (extra_perms ? CONCAT('u/', current_setting('session.user')));
CREATE POLICY see_extra_perms_user_insert ON sqs_trigger FOR INSERT TO windmill_user
WITH CHECK ((extra_perms ->> CONCAT('u/', current_setting('session.user')))::boolean);
CREATE POLICY see_extra_perms_user_update ON sqs_trigger FOR UPDATE TO windmill_user
USING ((extra_perms ->> CONCAT('u/', current_setting('session.user')))::boolean);
CREATE POLICY see_extra_perms_user_delete ON sqs_trigger FOR DELETE TO windmill_user
USING ((extra_perms ->> CONCAT('u/', current_setting('session.user')))::boolean);

CREATE POLICY see_extra_perms_groups_select ON sqs_trigger FOR SELECT TO windmill_user
USING (extra_perms ?| regexp_split_to_array(current_setting('session.pgroups'), ',')::text[]);
CREATE POLICY see_extra_perms_groups_insert ON sqs_trigger FOR INSERT TO windmill_user
WITH CHECK (exists(
    SELECT key, value FROM jsonb_each_text(extra_perms) 
    WHERE SPLIT_PART(key, '/', 1) = 'g' AND key = ANY(regexp_split_to_array(current_setting('session.pgroups'), ',')::text[])
    AND value::boolean));
CREATE POLICY see_extra_perms_groups_update ON sqs_trigger FOR UPDATE TO windmill_user
USING (exists(
    SELECT key, value FROM jsonb_each_text(extra_perms) 
    WHERE SPLIT_PART(key, '/', 1) = 'g' AND key = ANY(regexp_split_to_array(current_setting('session.pgroups'), ',')::text[])
    AND value::boolean));
CREATE POLICY see_extra_perms_groups_delete ON sqs_trigger FOR DELETE  TO windmill_user
USING (exists(
    SELECT key, value FROM jsonb_each_text(extra_perms) 
    WHERE SPLIT_PART(key, '/', 1) = 'g' AND key = ANY(regexp_split_to_array(current_setting('session.pgroups'), ',')::text[])
    AND value::boolean));