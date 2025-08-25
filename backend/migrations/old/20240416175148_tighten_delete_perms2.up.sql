
DO
$do$
  DECLARE
    i text;
    arr text[] := array['resource', 'script', 'variable', 'schedule', 'flow', 'app', 'raw_app'];
  BEGIN
  FOREACH i IN ARRAY arr
  LOOP
    EXECUTE FORMAT(
      $$
        DROP POLICY IF EXISTS see_folder_extra_perms_user_delete ON %1$I;

        CREATE POLICY see_folder_extra_perms_user_delete ON %1$I FOR DELETE TO windmill_user
        USING (SPLIT_PART(%1$I.path, '/', 1) = 'f' AND SPLIT_PART(%1$I.path, '/', 2) = any(regexp_split_to_array(current_setting('session.folders_write'), ',')::text[]));

      $$,
      i
    );
  END LOOP;
  END
$do$;-- Add up migration script here
