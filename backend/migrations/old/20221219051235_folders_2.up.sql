-- Add up migration script here
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
        CREATE POLICY see_folder_extra_perms_user ON %1$I FOR ALL
        USING (SPLIT_PART(%1$I.path, '/', 1) = 'f' AND SPLIT_PART(%1$I.path, '/', 2) = any(regexp_split_to_array(current_setting('session.folders_read'), ',')::text[]))
        WITH CHECK (SPLIT_PART(%1$I.path, '/', 1) = 'f' AND SPLIT_PART(%1$I.path, '/', 2) = any(regexp_split_to_array(current_setting('session.folders_write'), ',')::text[]));
      $$,
      i
    );
  END LOOP;
  END
$do$;
ALTER TABLE folder ENABLE ROW LEVEL SECURITY;

CREATE POLICY see_extra_perms_user ON folder FOR ALL
USING (extra_perms ? CONCAT('u/', current_setting('session.user')) or (CONCAT('u/', current_setting('session.user')) = ANY(owners)))
WITH CHECK ((CONCAT('u/', current_setting('session.user')) = ANY(owners)));

CREATE POLICY see_extra_perms_groups ON folder FOR ALL
USING (extra_perms ?| regexp_split_to_array(current_setting('session.pgroups'), ',')::text[] or (exists(
    SELECT o FROM unnest(owners) as o
    WHERE o = ANY(regexp_split_to_array(current_setting('session.pgroups'), ',')::text[]))))
WITH CHECK (exists(
    SELECT o FROM unnest(owners) as o
    WHERE o = ANY(regexp_split_to_array(current_setting('session.pgroups'), ',')::text[])));

DO
$do$
  DECLARE
    i text;
    arr text[] := array['script', 'flow', 'variable', 'resource', 'schedule'];
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
