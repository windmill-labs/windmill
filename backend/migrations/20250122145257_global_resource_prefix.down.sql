-- Add down migration script here
DO
$do$
  DECLARE
    i text;
    arr text[] := array['resource', 'variable'];
  BEGIN
  FOREACH i IN ARRAY arr
  LOOP
    EXECUTE FORMAT(
      $$
        DROP POLICY see_globally_shared_resource ON %1$I
      $$,
      i
    );
  END LOOP;
  END
$do$;

DO
$do$
  DECLARE
    i text;
    arr text[] := array['variable', 'resource'];
  BEGIN
  FOREACH i IN ARRAY arr
  LOOP
    EXECUTE FORMAT(
      $$
        ALTER TABLE %1$I DROP CONSTRAINT proper_id;
        ALTER TABLE %1$I ADD CONSTRAINT proper_id CHECK (path ~ '^[ufg](\/[\w-]+){2,}$');
      $$,
      i
    );
  END LOOP;
  END
$do$;
