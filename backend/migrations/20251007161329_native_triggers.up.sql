-- Add up migration script here

-- Create enum for native trigger service names
CREATE TYPE native_trigger_service AS ENUM ('nextcloud');
CREATE TYPE runnable_kind AS ENUM ('script', 'flow');


-- Create native_triggers table
CREATE TABLE native_triggers (
    service_name native_trigger_service NOT NULL,
    external_id VARCHAR(255) NOT NULL,
    runnable_path VARCHAR(255) NOT NULL,
    runnable_kind RUNNABLE_KIND NOT NULL,
    path VARCHAR(255) NOT NULL,
    workspace_id VARCHAR(50) NOT NULL,
    resource_path VARCHAR(255) NOT NULL,
    summary TEXT NOT NULL,
    metadata JSONB NULL,
    edited_by VARCHAR(50) NOT NULL,
    email VARCHAR(255) NOT NULL,
    edited_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    extra_perms JSONB NULL,
    CONSTRAINT pk_native_triggers PRIMARY KEY (workspace_id, path),
    CONSTRAINT uq_native_triggers_external UNIQUE (service_name, workspace_id, external_id),
    CONSTRAINT fk_native_triggers_workspace FOREIGN KEY (workspace_id)
        REFERENCES workspace(id) ON DELETE CASCADE
);

-- Create indexes
CREATE INDEX idx_native_triggers_service_workspace_external
    ON native_triggers (service_name, workspace_id, external_id);

CREATE INDEX idx_native_triggers_workspace_path
    ON native_triggers (workspace_id, path);

-- Grant permissions
GRANT ALL ON native_triggers TO windmill_user;
GRANT ALL ON native_triggers TO windmill_admin;

-- Enable row level security
ALTER TABLE native_triggers ENABLE ROW LEVEL SECURITY;

-- Create policies
CREATE POLICY admin_policy ON native_triggers FOR ALL TO windmill_admin USING (true);

CREATE POLICY see_folder_extra_perms_user_select ON native_triggers FOR SELECT TO windmill_user
USING (SPLIT_PART(native_triggers.path, '/', 1) = 'f' AND SPLIT_PART(native_triggers.path, '/', 2) = any(regexp_split_to_array(current_setting('session.folders_read'), ',')::text[]));

CREATE POLICY see_folder_extra_perms_user_insert ON native_triggers FOR INSERT TO windmill_user
WITH CHECK (SPLIT_PART(native_triggers.path, '/', 1) = 'f' AND SPLIT_PART(native_triggers.path, '/', 2) = any(regexp_split_to_array(current_setting('session.folders_write'), ',')::text[]));

CREATE POLICY see_folder_extra_perms_user_update ON native_triggers FOR UPDATE TO windmill_user
USING (SPLIT_PART(native_triggers.path, '/', 1) = 'f' AND SPLIT_PART(native_triggers.path, '/', 2) = any(regexp_split_to_array(current_setting('session.folders_write'), ',')::text[]));

CREATE POLICY see_folder_extra_perms_user_delete ON native_triggers FOR DELETE TO windmill_user
USING (SPLIT_PART(native_triggers.path, '/', 1) = 'f' AND SPLIT_PART(native_triggers.path, '/', 2) = any(regexp_split_to_array(current_setting('session.folders_write'), ',')::text[]));

CREATE POLICY see_own ON native_triggers FOR ALL TO windmill_user
USING (SPLIT_PART(native_triggers.path, '/', 1) = 'u' AND SPLIT_PART(native_triggers.path, '/', 2) = current_setting('session.user'));

CREATE POLICY see_member ON native_triggers FOR ALL TO windmill_user
USING (SPLIT_PART(native_triggers.path, '/', 1) = 'g' AND SPLIT_PART(native_triggers.path, '/', 2) = any(regexp_split_to_array(current_setting('session.groups'), ',')::text[]));

CREATE POLICY see_extra_perms_user_select ON native_triggers FOR SELECT TO windmill_user
USING (extra_perms ? CONCAT('u/', current_setting('session.user')));

CREATE POLICY see_extra_perms_user_insert ON native_triggers FOR INSERT TO windmill_user
WITH CHECK ((extra_perms ->> CONCAT('u/', current_setting('session.user')))::boolean);

CREATE POLICY see_extra_perms_user_update ON native_triggers FOR UPDATE TO windmill_user
USING ((extra_perms ->> CONCAT('u/', current_setting('session.user')))::boolean);

CREATE POLICY see_extra_perms_user_delete ON native_triggers FOR DELETE TO windmill_user
USING ((extra_perms ->> CONCAT('u/', current_setting('session.user')))::boolean);

CREATE POLICY see_extra_perms_groups_select ON native_triggers FOR SELECT TO windmill_user
USING (extra_perms ?| regexp_split_to_array(current_setting('session.pgroups'), ',')::text[]);

CREATE POLICY see_extra_perms_groups_insert ON native_triggers FOR INSERT TO windmill_user
WITH CHECK (exists(
    SELECT key, value FROM jsonb_each_text(extra_perms)
    WHERE SPLIT_PART(key, '/', 1) = 'g' AND key = ANY(regexp_split_to_array(current_setting('session.pgroups'), ',')::text[])
    AND value::boolean));

CREATE POLICY see_extra_perms_groups_update ON native_triggers FOR UPDATE TO windmill_user
USING (exists(
    SELECT key, value FROM jsonb_each_text(extra_perms)
    WHERE SPLIT_PART(key, '/', 1) = 'g' AND key = ANY(regexp_split_to_array(current_setting('session.pgroups'), ',')::text[])
    AND value::boolean));

CREATE POLICY see_extra_perms_groups_delete ON native_triggers FOR DELETE TO windmill_user
USING (exists(
    SELECT key, value FROM jsonb_each_text(extra_perms)
    WHERE SPLIT_PART(key, '/', 1) = 'g' AND key = ANY(regexp_split_to_array(current_setting('session.pgroups'), ',')::text[])
    AND value::boolean));

-- Add nextcloud to job_trigger_kind enum
ALTER TYPE job_trigger_kind ADD VALUE IF NOT EXISTS 'nextcloud';
