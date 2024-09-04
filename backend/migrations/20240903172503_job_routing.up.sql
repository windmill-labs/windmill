-- Add up migration script here
CREATE TYPE trigger_kind AS ENUM ('http', 'email');

CREATE TABLE trigger (
  path VARCHAR(255) NOT NULL,
  kind trigger_kind NOT NULL,
  route_path VARCHAR(255) NOT NULL,
  job_path VARCHAR(255) NOT NULL,
  is_flow BOOLEAN NOT NULL,
  workspace_id VARCHAR(50) NOT NULL,
  edited_by VARCHAR(50) NOT NULL,
  edited_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  extra_perms JSONB NOT NULL DEFAULT '{}',
  PRIMARY KEY (path, workspace_id)
);


GRANT ALL ON trigger TO windmill_user;
GRANT ALL ON trigger TO windmill_admin;

CREATE POLICY see_folder_extra_perms_user_select ON trigger FOR SELECT TO windmill_user
USING (SPLIT_PART(trigger.path, '/', 1) = 'f' AND SPLIT_PART(trigger.path, '/', 2) = any(regexp_split_to_array(current_setting('session.folders_read'), ',')::text[]));
CREATE POLICY see_folder_extra_perms_user_insert ON trigger FOR INSERT TO windmill_user
WITH CHECK (SPLIT_PART(trigger.path, '/', 1) = 'f' AND SPLIT_PART(trigger.path, '/', 2) = any(regexp_split_to_array(current_setting('session.folders_write'), ',')::text[]));
CREATE POLICY see_folder_extra_perms_user_update ON trigger FOR UPDATE TO windmill_user
USING (SPLIT_PART(trigger.path, '/', 1) = 'f' AND SPLIT_PART(trigger.path, '/', 2) = any(regexp_split_to_array(current_setting('session.folders_write'), ',')::text[]));
CREATE POLICY see_folder_extra_perms_user_delete ON trigger FOR UPDATE TO windmill_user
USING (SPLIT_PART(trigger.path, '/', 1) = 'f' AND SPLIT_PART(trigger.path, '/', 2) = any(regexp_split_to_array(current_setting('session.folders_write'), ',')::text[]));

CREATE POLICY see_own ON trigger FOR ALL TO windmill_user
USING (SPLIT_PART(trigger.path, '/', 1) = 'u' AND SPLIT_PART(trigger.path, '/', 2) = current_setting('session.user'));
CREATE POLICY see_member ON trigger FOR ALL TO windmill_user
USING (SPLIT_PART(trigger.path, '/', 1) = 'g' AND SPLIT_PART(trigger.path, '/', 2) = any(regexp_split_to_array(current_setting('session.groups'), ',')::text[]));

CREATE POLICY see_extra_perms_user_select ON trigger FOR SELECT TO windmill_user
USING (extra_perms ? CONCAT('u/', current_setting('session.user')));
CREATE POLICY see_extra_perms_user_insert ON trigger FOR INSERT TO windmill_user
WITH CHECK ((extra_perms ->> CONCAT('u/', current_setting('session.user')))::boolean);
CREATE POLICY see_extra_perms_user_update ON trigger FOR UPDATE TO windmill_user
USING ((extra_perms ->> CONCAT('u/', current_setting('session.user')))::boolean);
CREATE POLICY see_extra_perms_user_delete ON trigger FOR DELETE TO windmill_user
USING ((extra_perms ->> CONCAT('u/', current_setting('session.user')))::boolean);

CREATE POLICY see_extra_perms_groups_select ON trigger FOR SELECT TO windmill_user
USING (extra_perms ?| regexp_split_to_array(current_setting('session.pgroups'), ',')::text[]);
CREATE POLICY see_extra_perms_groups_insert ON trigger FOR INSERT TO windmill_user
WITH CHECK (exists(
    SELECT key, value FROM jsonb_each_text(extra_perms) 
    WHERE SPLIT_PART(key, '/', 1) = 'g' AND key = ANY(regexp_split_to_array(current_setting('session.pgroups'), ',')::text[])
    AND value::boolean));
CREATE POLICY see_extra_perms_groups_update ON trigger FOR UPDATE TO windmill_user
USING (exists(
    SELECT key, value FROM jsonb_each_text(extra_perms) 
    WHERE SPLIT_PART(key, '/', 1) = 'g' AND key = ANY(regexp_split_to_array(current_setting('session.pgroups'), ',')::text[])
    AND value::boolean));
CREATE POLICY see_extra_perms_groups_delete ON trigger FOR DELETE  TO windmill_user
USING (exists(
    SELECT key, value FROM jsonb_each_text(extra_perms) 
    WHERE SPLIT_PART(key, '/', 1) = 'g' AND key = ANY(regexp_split_to_array(current_setting('session.pgroups'), ',')::text[])
    AND value::boolean));

ALTER TABLE script ADD COLUMN has_preprocessor BOOLEAN;

ALTER TYPE job_kind ADD VALUE IF NOT EXISTS 'scriptwithpreprocessor';