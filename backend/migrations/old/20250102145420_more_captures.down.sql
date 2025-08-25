-- Add down migration script here
DROP TABLE capture_config;
DELETE FROM capture;
DROP POLICY see_from_allowed_runnables ON capture;
ALTER TABLE capture DROP CONSTRAINT capture_pkey;
ALTER TABLE capture DROP COLUMN is_flow, DROP COLUMN trigger_kind, DROP COLUMN trigger_extra, DROP COLUMN id;
ALTER TABLE capture ADD CONSTRAINT capture_pkey PRIMARY KEY (workspace_id, path);
DROP TYPE TRIGGER_KIND;

CREATE POLICY see_folder_extra_perms_user ON capture FOR ALL TO windmill_user
USING (SPLIT_PART(capture.path, '/', 1) = 'f' AND SPLIT_PART(capture.path, '/', 2) = any(regexp_split_to_array(current_setting('session.folders_read'), ',')::text[]))
WITH CHECK (SPLIT_PART(capture.path, '/', 1) = 'f' AND SPLIT_PART(capture.path, '/', 2) = any(regexp_split_to_array(current_setting('session.folders_write'), ',')::text[]));
CREATE POLICY see_member ON public.capture TO windmill_user USING (((split_part((path)::text, '/'::text, 1) = 'g'::text) AND (split_part((path)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.groups'::text), ','::text)))));
CREATE POLICY see_own ON public.capture TO windmill_user USING (((split_part((path)::text, '/'::text, 1) = 'u'::text) AND (split_part((path)::text, '/'::text, 2) = current_setting('session.user'::text))));
