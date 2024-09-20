-- Add up migration script here
CREATE TYPE http_method AS ENUM ('get', 'post', 'put', 'delete', 'patch');

CREATE TABLE http_trigger (
  path VARCHAR(255) NOT NULL,
  route_path VARCHAR(255) NOT NULL,
  route_path_key VARCHAR(255) NOT NULL,
  script_path VARCHAR(255) NOT NULL,
  is_flow BOOLEAN NOT NULL,
  workspace_id VARCHAR(50) NOT NULL,
  edited_by VARCHAR(50) NOT NULL,
  email VARCHAR(255) NOT NULL,
  edited_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  extra_perms JSONB NOT NULL DEFAULT '{}',
  is_async BOOLEAN NOT NULL DEFAULT FALSE,
  requires_auth BOOLEAN NOT NULL DEFAULT FALSE,
  http_method http_method NOT NULL,
  PRIMARY KEY (path, workspace_id)
);


GRANT SELECT, UPDATE ON http_trigger TO windmill_user;
GRANT ALL ON http_trigger TO windmill_admin;

ALTER TABLE http_trigger ENABLE ROW LEVEL SECURITY;

CREATE POLICY see_folder_extra_perms_user_select ON http_trigger FOR SELECT TO windmill_user
USING (SPLIT_PART(http_trigger.path, '/', 1) = 'f' AND SPLIT_PART(http_trigger.path, '/', 2) = any(regexp_split_to_array(current_setting('session.folders_read'), ',')::text[]));
CREATE POLICY see_folder_extra_perms_user_update ON http_trigger FOR UPDATE TO windmill_user
USING (SPLIT_PART(http_trigger.path, '/', 1) = 'f' AND SPLIT_PART(http_trigger.path, '/', 2) = any(regexp_split_to_array(current_setting('session.folders_write'), ',')::text[]));


CREATE POLICY see_extra_perms_user_select ON http_trigger FOR SELECT TO windmill_user
USING (extra_perms ? CONCAT('u/', current_setting('session.user')));
CREATE POLICY see_extra_perms_user_update ON http_trigger FOR UPDATE TO windmill_user
USING ((extra_perms ->> CONCAT('u/', current_setting('session.user')))::boolean);

CREATE POLICY see_extra_perms_groups_select ON http_trigger FOR SELECT TO windmill_user
USING (extra_perms ?| regexp_split_to_array(current_setting('session.pgroups'), ',')::text[]);
CREATE POLICY see_extra_perms_groups_update ON http_trigger FOR UPDATE TO windmill_user
USING (exists(
    SELECT key, value FROM jsonb_each_text(extra_perms) 
    WHERE SPLIT_PART(key, '/', 1) = 'g' AND key = ANY(regexp_split_to_array(current_setting('session.pgroups'), ',')::text[])
    AND value::boolean));

CREATE OR REPLACE FUNCTION prevent_route_path_change()
RETURNS TRIGGER AS $$
BEGIN
    IF CURRENT_USER <> 'windmill_admin' AND NEW.route_path <> OLD.route_path THEN
        RAISE EXCEPTION 'Modification of route_path is only allowed by admins';
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER check_route_path_change
BEFORE UPDATE ON http_trigger
FOR EACH ROW
EXECUTE FUNCTION prevent_route_path_change();

ALTER TABLE script ADD COLUMN has_preprocessor BOOLEAN;
