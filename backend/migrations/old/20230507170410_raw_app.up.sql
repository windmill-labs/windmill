-- Add up migration script here
CREATE TABLE raw_app (
    path varchar(255) PRIMARY KEY,
    version INTEGER NOT NULL DEFAULT 0,
    workspace_id VARCHAR(50) NOT NULL REFERENCES workspace(id),
    summary VARCHAR(1000) NOT NULL DEFAULT '',
    edited_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT now(),
    data TEXT NOT NULL,
    extra_perms JSONB NOT NULL DEFAULT '{}'
);

CREATE POLICY see_own ON raw_app FOR ALL
USING (SPLIT_PART(raw_app.path, '/', 1) = 'u' AND SPLIT_PART(raw_app.path, '/', 2) = current_setting('session.user'));

CREATE POLICY see_member ON raw_app FOR ALL
USING (SPLIT_PART(raw_app.path, '/', 1) = 'g' AND SPLIT_PART(raw_app.path, '/', 2) = any(regexp_split_to_array(current_setting('session.groups'), ',')::text[]));

CREATE POLICY see_extra_perms_user ON raw_app FOR ALL
USING (extra_perms ? CONCAT('u/', current_setting('session.user')))
WITH CHECK ((extra_perms ->> CONCAT('u/', current_setting('session.user')))::boolean);

CREATE POLICY see_extra_perms_groups ON raw_app FOR ALL
USING (extra_perms ?| regexp_split_to_array(current_setting('session.pgroups'), ',')::text[])
WITH CHECK (exists(
    SELECT key, value FROM jsonb_each_text(extra_perms) 
    WHERE SPLIT_PART(key, '/', 1) = 'g' AND key = ANY(regexp_split_to_array(current_setting('session.pgroups'), ',')::text[])
    AND value::boolean));

CREATE POLICY see_folder_extra_perms_user ON raw_app FOR ALL
USING (SPLIT_PART(raw_app.path, '/', 1) = 'f' AND SPLIT_PART(raw_app.path, '/', 2) = any(regexp_split_to_array(current_setting('session.folders_read'), ',')::text[]))
WITH CHECK (SPLIT_PART(raw_app.path, '/', 1) = 'f' AND SPLIT_PART(raw_app.path, '/', 2) = any(regexp_split_to_array(current_setting('session.folders_write'), ',')::text[]));

ALTER TYPE FAVORITE_KIND ADD VALUE 'raw_app';