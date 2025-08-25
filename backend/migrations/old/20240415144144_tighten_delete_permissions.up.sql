
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
        DROP POLICY IF EXISTS see_folder_extra_perms_user ON %1$I;
        DROP POLICY IF EXISTS see_folder_extra_perms_user_delete ON %1$I;
        DROP POLICY IF EXISTS see_extra_perms_user ON %1$I;
        DROP POLICY IF EXISTS see_member ON %1$I;
        DROP POLICY IF EXISTS see_own ON %1$I;
        DROP POLICY IF EXISTS see_extra_perms_user_delete ON %1$I;
        DROP POLICY IF EXISTS see_extra_perms_groups ON %1$I;
        DROP POLICY IF EXISTS see_extra_perms_groups_delete ON %1$I;

        -- New policies for select, insert, update
        DROP POLICY IF EXISTS see_folder_extra_perms_user_select ON %1$I;
        DROP POLICY IF EXISTS see_folder_extra_perms_user_insert ON %1$I;
        DROP POLICY IF EXISTS see_folder_extra_perms_user_update ON %1$I;

        DROP POLICY IF EXISTS see_own ON %1$I;
        DROP POLICY IF EXISTS see_member ON %1$I;

        DROP POLICY IF EXISTS see_extra_perms_user_select ON %1$I;
        DROP POLICY IF EXISTS see_extra_perms_user_insert ON %1$I;
        DROP POLICY IF EXISTS see_extra_perms_user_update ON %1$I;

        DROP POLICY IF EXISTS see_extra_perms_groups_select ON %1$I;
        DROP POLICY IF EXISTS see_extra_perms_groups_insert ON %1$I;
        DROP POLICY IF EXISTS see_extra_perms_groups_update ON %1$I;


        -- Folder permissions split into select, insert, and update
        CREATE POLICY see_folder_extra_perms_user_select ON %1$I FOR SELECT TO windmill_user
        USING (SPLIT_PART(%1$I.path, '/', 1) = 'f' AND SPLIT_PART(%1$I.path, '/', 2) = any(regexp_split_to_array(current_setting('session.folders_read'), ',')::text[]));

        CREATE POLICY see_folder_extra_perms_user_insert ON %1$I FOR INSERT TO windmill_user
        WITH CHECK (SPLIT_PART(%1$I.path, '/', 1) = 'f' AND SPLIT_PART(%1$I.path, '/', 2) = any(regexp_split_to_array(current_setting('session.folders_write'), ',')::text[]));

        CREATE POLICY see_folder_extra_perms_user_update ON %1$I FOR UPDATE TO windmill_user
        USING (SPLIT_PART(%1$I.path, '/', 1) = 'f' AND SPLIT_PART(%1$I.path, '/', 2) = any(regexp_split_to_array(current_setting('session.folders_write'), ',')::text[]));

        CREATE POLICY see_folder_extra_perms_user_delete ON %1$I FOR UPDATE TO windmill_user
        USING (SPLIT_PART(%1$I.path, '/', 1) = 'f' AND SPLIT_PART(%1$I.path, '/', 2) = any(regexp_split_to_array(current_setting('session.folders_write'), ',')::text[]));

        CREATE POLICY see_own ON %1$I FOR ALL TO windmill_user
        USING (SPLIT_PART(%1$I.path, '/', 1) = 'u' AND SPLIT_PART(%1$I.path, '/', 2) = current_setting('session.user'));


        CREATE POLICY see_member ON %1$I FOR ALL TO windmill_user
        USING (SPLIT_PART(%1$I.path, '/', 1) = 'g' AND SPLIT_PART(%1$I.path, '/', 2) = any(regexp_split_to_array(current_setting('session.groups'), ',')::text[]));



        CREATE POLICY see_extra_perms_user_select ON %1$I FOR SELECT TO windmill_user
        USING (extra_perms ? CONCAT('u/', current_setting('session.user')));

        CREATE POLICY see_extra_perms_user_insert ON %1$I FOR INSERT TO windmill_user
        WITH CHECK ((extra_perms ->> CONCAT('u/', current_setting('session.user')))::boolean);

        CREATE POLICY see_extra_perms_user_update ON %1$I FOR UPDATE TO windmill_user
        USING ((extra_perms ->> CONCAT('u/', current_setting('session.user')))::boolean);

        CREATE POLICY see_extra_perms_user_delete ON %1$I FOR DELETE TO windmill_user
        USING ((extra_perms ->> CONCAT('u/', current_setting('session.user')))::boolean);




        CREATE POLICY see_extra_perms_groups_select ON %1$I FOR SELECT TO windmill_user
        USING (extra_perms ?| regexp_split_to_array(current_setting('session.pgroups'), ',')::text[]);

        CREATE POLICY see_extra_perms_groups_insert ON %1$I FOR INSERT TO windmill_user
        WITH CHECK (exists(
            SELECT key, value FROM jsonb_each_text(extra_perms) 
            WHERE SPLIT_PART(key, '/', 1) = 'g' AND key = ANY(regexp_split_to_array(current_setting('session.pgroups'), ',')::text[])
            AND value::boolean));

        CREATE POLICY see_extra_perms_groups_update ON %1$I FOR UPDATE TO windmill_user
        USING (exists(
            SELECT key, value FROM jsonb_each_text(extra_perms) 
            WHERE SPLIT_PART(key, '/', 1) = 'g' AND key = ANY(regexp_split_to_array(current_setting('session.pgroups'), ',')::text[])
            AND value::boolean));

        CREATE POLICY see_extra_perms_groups_delete ON %1$I FOR DELETE  TO windmill_user
        USING (exists(
            SELECT key, value FROM jsonb_each_text(extra_perms) 
            WHERE SPLIT_PART(key, '/', 1) = 'g' AND key = ANY(regexp_split_to_array(current_setting('session.pgroups'), ',')::text[])
            AND value::boolean));

      $$,
      i
    );
  END LOOP;
  END
$do$;

DROP POLICY IF EXISTS see_extra_perms_user ON folder;
DROP POLICY IF EXISTS see_extra_perms_user_select ON folder;
DROP POLICY IF EXISTS see_extra_perms_user_insert ON folder;
DROP POLICY IF EXISTS see_extra_perms_user_update ON folder;
DROP POLICY IF EXISTS see_extra_perms_user_delete ON folder;

DROP POLICY IF EXISTS see_extra_perms_groups ON folder;
DROP POLICY IF EXISTS see_extra_perms_groups_select ON folder;
DROP POLICY IF EXISTS see_extra_perms_groups_insert ON folder;
DROP POLICY IF EXISTS see_extra_perms_groups_update ON folder;
DROP POLICY IF EXISTS see_extra_perms_groups_delete ON folder;


-- Existing CREATE POLICY statements updated to reflect policy splitting for 'folder' table
CREATE POLICY see_extra_perms_user_select ON folder FOR SELECT TO windmill_user
USING (extra_perms ? CONCAT('u/', current_setting('session.user')) OR CONCAT('u/', current_setting('session.user')) = ANY(owners));

CREATE POLICY see_extra_perms_user_insert ON folder FOR INSERT TO windmill_user
WITH CHECK ((CONCAT('u/', current_setting('session.user')) = ANY(owners)));

CREATE POLICY see_extra_perms_user_update ON folder FOR UPDATE TO windmill_user
USING ((CONCAT('u/', current_setting('session.user')) = ANY(owners)));

CREATE POLICY see_extra_perms_user_delete ON folder FOR DELETE TO windmill_user
USING ((CONCAT('u/', current_setting('session.user')) = ANY(owners)));

CREATE POLICY see_extra_perms_groups_select ON folder FOR SELECT TO windmill_user
USING (extra_perms ?| regexp_split_to_array(current_setting('session.pgroups'), ',')::text[] OR EXISTS (
    SELECT o FROM unnest(owners) AS o
    WHERE o = ANY(regexp_split_to_array(current_setting('session.pgroups'), ',')::text[])));

CREATE POLICY see_extra_perms_groups_insert ON folder FOR INSERT TO windmill_user
WITH CHECK (EXISTS (
    SELECT o FROM unnest(owners) AS o
    WHERE o = ANY(regexp_split_to_array(current_setting('session.pgroups'), ',')::text[])));


CREATE POLICY see_extra_perms_groups_update ON folder FOR UPDATE TO windmill_user
USING (EXISTS (
    SELECT o FROM unnest(owners) AS o
    WHERE o = ANY(regexp_split_to_array(current_setting('session.pgroups'), ',')::text[])));


CREATE POLICY see_extra_perms_groups_delete ON folder FOR DELETE TO windmill_user
USING (EXISTS (
    SELECT o FROM unnest(owners) AS o
    WHERE o = ANY(regexp_split_to_array(current_setting('session.pgroups'), ',')::text[])));


-- DO
-- $do$
--   DECLARE
--     i text;
--     arr text[] := array['resource', 'script', 'variable', 'schedule', 'flow', 'app', 'raw_app'];
--   BEGIN
--   FOREACH i IN ARRAY arr
--   LOOP
--     EXECUTE FORMAT(
--       $$


--         CREATE POLICY see_folder_extra_perms_user ON %1$I FOR ALL TO windmill_user
--         USING (SPLIT_PART(%1$I.path, '/', 1) = 'f' AND SPLIT_PART(%1$I.path, '/', 2) = any(regexp_split_to_array(current_setting('session.folders_read'), ',')::text[]))
--         WITH CHECK (SPLIT_PART(%1$I.path, '/', 1) = 'f' AND SPLIT_PART(%1$I.path, '/', 2) = any(regexp_split_to_array(current_setting('session.folders_write'), ',')::text[]));

--         CREATE POLICY see_folder_extra_perms_user_delete ON %1$I AS RESTRICTIVE FOR DELETE TO windmill_user
--         USING (SPLIT_PART(%1$I.path, '/', 1) = 'f' AND SPLIT_PART(%1$I.path, '/', 2) = any(regexp_split_to_array(current_setting('session.folders_write'), ',')::text[]));

--         CREATE POLICY see_own ON %1$I FOR ALL TO windmill_user
--         USING (SPLIT_PART(%1$I.path, '/', 1) = 'u' AND SPLIT_PART(%1$I.path, '/', 2) = current_setting('session.user'));

--         CREATE POLICY see_member ON %1$I FOR ALL TO windmill_user
--         USING (SPLIT_PART(%1$I.path, '/', 1) = 'g' AND SPLIT_PART(%1$I.path, '/', 2) = any(regexp_split_to_array(current_setting('session.groups'), ',')::text[]));


--         CREATE POLICY see_extra_perms_user ON %1$I FOR ALL TO windmill_user
--         USING (extra_perms ? CONCAT('u/', current_setting('session.user')))
--         WITH CHECK ((extra_perms ->> CONCAT('u/', current_setting('session.user')))::boolean);

--         CREATE POLICY see_extra_perms_user_delete ON %1$I FOR DELETE TO windmill_user
--         USING ((extra_perms ->> CONCAT('u/', current_setting('session.user')))::boolean);

--         CREATE POLICY see_extra_perms_groups ON %1$I FOR ALL TO windmill_user
--         USING (extra_perms ?| regexp_split_to_array(current_setting('session.pgroups'), ',')::text[])
--         WITH CHECK (exists(
--             SELECT key, value FROM jsonb_each_text(extra_perms) 
--             WHERE SPLIT_PART(key, '/', 1) = 'g' AND key = ANY(regexp_split_to_array(current_setting('session.pgroups'), ',')::text[])
--             AND value::boolean));

--         CREATE POLICY see_extra_perms_groups_delete ON %1$I FOR DELETE  TO windmill_user
--         USING (exists(
--             SELECT key, value FROM jsonb_each_text(extra_perms) 
--             WHERE SPLIT_PART(key, '/', 1) = 'g' AND key = ANY(regexp_split_to_array(current_setting('session.pgroups'), ',')::text[])
--             AND value::boolean));
--       $$,
--       i
--     );
--   END LOOP;
--   END
-- $do$;


-- DROP POLICY see_extra_perms_user ON folder;
-- DROP POLICY see_extra_perms_groups ON folder;

-- CREATE POLICY see_extra_perms_user ON folder FOR ALL to windmill_user
-- USING (extra_perms ? CONCAT('u/', current_setting('session.user')) or (CONCAT('u/', current_setting('session.user')) = ANY(owners)))
-- WITH CHECK ((CONCAT('u/', current_setting('session.user')) = ANY(owners)));

-- DROP POLICY IF EXISTS see_extra_perms_user_delete ON folder;
-- CREATE POLICY see_extra_perms_user_delete ON folder AS RESTRICTIVE FOR DELETE to windmill_user
-- USING ((CONCAT('u/', current_setting('session.user')) = ANY(owners)));

-- CREATE POLICY see_extra_perms_groups ON folder FOR ALL to windmill_user
-- USING (extra_perms ?| regexp_split_to_array(current_setting('session.pgroups'), ',')::text[] or (exists(
--     SELECT o FROM unnest(owners) as o
--     WHERE o = ANY(regexp_split_to_array(current_setting('session.pgroups'), ',')::text[]))))
-- WITH CHECK (exists(
--     SELECT o FROM unnest(owners) as o
--     WHERE o = ANY(regexp_split_to_array(current_setting('session.pgroups'), ',')::text[])));

-- DROP POLICY IF EXISTS see_extra_perms_groups_delete ON folder;
-- CREATE POLICY see_extra_perms_groups_delete ON folder AS RESTRICTIVE FOR DELETE to windmill_user
-- USING (exists(
--     SELECT o FROM unnest(owners) as o
--     WHERE o = ANY(regexp_split_to_array(current_setting('session.pgroups'), ',')::text[])));