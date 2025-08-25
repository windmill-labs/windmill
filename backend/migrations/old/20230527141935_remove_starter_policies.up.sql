-- Add up migration script here
DO
$do$
  DECLARE
    i text;
    arr text[] := array['resource', 'script', 'variable', 'schedule', 'flow', 'completed_job'];
  BEGIN
  FOREACH i IN ARRAY arr
  LOOP
    EXECUTE FORMAT(
      $$
      DROP POLICY see_starter ON %1$I;
      $$,
      i
    );
  END LOOP;
  END
$do$;
