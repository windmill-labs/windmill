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
        CREATE POLICY see_globally_shared_resource ON %1$I FOR SELECT
        USING (SPLIT_PART(%1$I.path, '/', 1) = 's');
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
        ALTER TABLE %1$I ADD CONSTRAINT proper_id CHECK (path ~ '^[ufgs](\/[\w-]+){2,}$');
      $$,
      i
    );
  END LOOP;
  END
$do$;
