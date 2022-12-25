-- Add down migration script here
DO
$do$
  DECLARE
    i text;
    arr text[] := array['resource', 'script', 'variable', 'schedule', 'flow', 'app'];
  BEGIN
  FOREACH i IN ARRAY arr
  LOOP
    EXECUTE FORMAT(
      $$
        DROP POLICY see_folder_extra_perms_user ON %1$I;
      $$,
      i
    );
  END LOOP;
  END
$do$;

DROP POLICY see_extra_perms_user ON folder;
DROP POLICY see_extra_perms_groups ON folder;