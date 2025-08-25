-- Add up migration script here


CREATE POLICY see_folder_extra_perms_user ON capture FOR ALL
USING (SPLIT_PART(capture.path, '/', 1) = 'f' AND SPLIT_PART(capture.path, '/', 2) = any(regexp_split_to_array(current_setting('session.folders_read'), ',')::text[]))
WITH CHECK (SPLIT_PART(capture.path, '/', 1) = 'f' AND SPLIT_PART(capture.path, '/', 2) = any(regexp_split_to_array(current_setting('session.folders_write'), ',')::text[]));

DO
$do$
  DECLARE
    i text;
    arr text[] := array['queue', 'completed_job'];
  BEGIN
  FOREACH i IN ARRAY arr
  LOOP
    EXECUTE FORMAT(
      $$
        CREATE POLICY see_folder_extra_perms_user ON %1$I FOR ALL
        USING (%1$I.visible_to_owner IS true AND SPLIT_PART(%1$I.script_path, '/', 1) = 'f' AND SPLIT_PART(%1$I.script_path, '/', 2) = any(regexp_split_to_array(current_setting('session.folders_read'), ',')::text[]));
      $$,
      i
    );
  END LOOP;
  END
$do$;