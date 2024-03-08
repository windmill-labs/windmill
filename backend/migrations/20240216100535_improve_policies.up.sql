-- Add up migration script here


DROP POLICY see_folder_extra_perms_user ON capture;
CREATE POLICY see_folder_extra_perms_user ON capture FOR ALL TO windmill_user
USING (SPLIT_PART(capture.path, '/', 1) = 'f' AND SPLIT_PART(capture.path, '/', 2) = any(regexp_split_to_array(current_setting('session.folders_read'), ',')::text[]))
WITH CHECK (SPLIT_PART(capture.path, '/', 1) = 'f' AND SPLIT_PART(capture.path, '/', 2) = any(regexp_split_to_array(current_setting('session.folders_write'), ',')::text[]));

DROP POLICY see_own ON capture;
CREATE POLICY see_own ON public.capture TO windmill_user USING (((split_part((path)::text, '/'::text, 1) = 'u'::text) AND (split_part((path)::text, '/'::text, 2) = current_setting('session.user'::text))));

DROP POLICY see_member ON capture;
CREATE POLICY see_member ON public.capture TO windmill_user USING (((split_part((path)::text, '/'::text, 1) = 'g'::text) AND (split_part((path)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.groups'::text), ','::text)))));

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
        DROP POLICY IF EXISTS see_folder_extra_perms_user ON %1$I;
        CREATE POLICY see_folder_extra_perms_user ON %1$I FOR ALL TO windmill_user
        USING (%1$I.visible_to_owner IS true AND SPLIT_PART(%1$I.script_path, '/', 1) = 'f' AND SPLIT_PART(%1$I.script_path, '/', 2) = any(regexp_split_to_array(current_setting('session.folders_read'), ',')::text[]));
      $$,
      i
    );
  END LOOP;
  END
$do$;



DROP POLICY see_own ON audit;
CREATE POLICY see_own ON audit FOR all TO windmill_user
      USING (((username)::text = current_setting('session.user'::text)));


DROP POLICY see_own_path ON queue;
DROP POLICY see_member_path ON queue;
DROP POLICY see_own ON queue;
DROP POLICY see_member ON queue;
DROP POLICY see_own_path ON completed_job;
DROP POLICY see_member_path ON completed_job;
DROP POLICY see_own ON completed_job;
DROP POLICY see_member ON completed_job;

CREATE POLICY see_own_path ON queue FOR ALL TO windmill_user
USING (queue.visible_to_owner IS true AND SPLIT_PART(queue.script_path, '/', 1) = 'u' AND SPLIT_PART(queue.script_path, '/', 2) = current_setting('session.user'));

CREATE POLICY see_member_path ON queue FOR ALL TO windmill_user
USING (queue.visible_to_owner IS true AND SPLIT_PART(queue.script_path, '/', 1) = 'g' AND SPLIT_PART(queue.script_path, '/', 2) = any(regexp_split_to_array(current_setting('session.groups'), ',')::text[]));

CREATE POLICY see_own ON queue FOR ALL TO windmill_user
USING (SPLIT_PART(queue.permissioned_as, '/', 1) = 'u' AND SPLIT_PART(queue.permissioned_as, '/', 2) = current_setting('session.user'));

CREATE POLICY see_member ON queue FOR ALL TO windmill_user
USING (SPLIT_PART(queue.permissioned_as, '/', 1) = 'g' AND SPLIT_PART(queue.permissioned_as, '/', 2) = any(regexp_split_to_array(current_setting('session.groups'), ',')::text[]));



CREATE POLICY see_own_path ON completed_job FOR ALL TO windmill_user
USING (completed_job.visible_to_owner IS true AND SPLIT_PART(completed_job.script_path, '/', 1) = 'u' AND SPLIT_PART(completed_job.script_path, '/', 2) = current_setting('session.user'));

CREATE POLICY see_member_path ON completed_job FOR ALL TO windmill_user
USING (completed_job.visible_to_owner IS true AND SPLIT_PART(completed_job.script_path, '/', 1) = 'g' AND SPLIT_PART(completed_job.script_path, '/', 2) = any(regexp_split_to_array(current_setting('session.groups'), ',')::text[]));

CREATE POLICY see_own ON completed_job FOR ALL TO windmill_user
USING (SPLIT_PART(completed_job.permissioned_as, '/', 1) = 'u' AND SPLIT_PART(completed_job.permissioned_as, '/', 2) = current_setting('session.user'));

CREATE POLICY see_member ON completed_job FOR ALL TO windmill_user
USING (SPLIT_PART(completed_job.permissioned_as, '/', 1) = 'g' AND SPLIT_PART(completed_job.permissioned_as, '/', 2) = any(regexp_split_to_array(current_setting('session.groups'), ',')::text[]));



DROP POLICY schedule on audit;
CREATE POLICY schedule ON audit FOR INSERT  TO windmill_user WITH CHECK (((username)::text ~~ 'schedule-%'::text));



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
        CREATE POLICY see_folder_extra_perms_user ON %1$I FOR ALL TO windmill_user
        USING (SPLIT_PART(%1$I.path, '/', 1) = 'f' AND SPLIT_PART(%1$I.path, '/', 2) = any(regexp_split_to_array(current_setting('session.folders_read'), ',')::text[]))
        WITH CHECK (SPLIT_PART(%1$I.path, '/', 1) = 'f' AND SPLIT_PART(%1$I.path, '/', 2) = any(regexp_split_to_array(current_setting('session.folders_write'), ',')::text[]));
      $$,
      i
    );
  END LOOP;
  END
$do$;

DROP POLICY see_extra_perms_user ON folder;
DROP POLICY see_extra_perms_groups ON folder;

CREATE POLICY see_extra_perms_user ON folder FOR ALL to windmill_user
USING (extra_perms ? CONCAT('u/', current_setting('session.user')) or (CONCAT('u/', current_setting('session.user')) = ANY(owners)))
WITH CHECK ((CONCAT('u/', current_setting('session.user')) = ANY(owners)));

CREATE POLICY see_extra_perms_groups ON folder FOR ALL to windmill_user
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
    arr text[] := array['resource', 'script', 'variable', 'schedule', 'flow', 'app', 'raw_app'];
  BEGIN
  FOREACH i IN ARRAY arr
  LOOP
    EXECUTE FORMAT(
      $$

        DROP POLICY IF EXISTS see_own ON %1$I;
        CREATE POLICY see_own ON %1$I FOR ALL TO windmill_user
        USING (SPLIT_PART(%1$I.path, '/', 1) = 'u' AND SPLIT_PART(%1$I.path, '/', 2) = current_setting('session.user'));

        DROP POLICY IF EXISTS see_member ON %1$I;
        CREATE POLICY see_member ON %1$I FOR ALL TO windmill_user
        USING (SPLIT_PART(%1$I.path, '/', 1) = 'g' AND SPLIT_PART(%1$I.path, '/', 2) = any(regexp_split_to_array(current_setting('session.groups'), ',')::text[]));


        DROP POLICY IF EXISTS see_extra_perms_user ON %1$I;
        CREATE POLICY see_extra_perms_user ON %1$I FOR ALL TO windmill_user
        USING (extra_perms ? CONCAT('u/', current_setting('session.user')))
        WITH CHECK ((extra_perms ->> CONCAT('u/', current_setting('session.user')))::boolean);

        DROP POLICY IF EXISTS see_extra_perms_groups ON %1$I;
        CREATE POLICY see_extra_perms_groups ON %1$I FOR ALL TO windmill_user
        USING (extra_perms ?| regexp_split_to_array(current_setting('session.pgroups'), ',')::text[])
        WITH CHECK (exists(
            SELECT key, value FROM jsonb_each_text(extra_perms) 
            WHERE SPLIT_PART(key, '/', 1) = 'g' AND key = ANY(regexp_split_to_array(current_setting('session.pgroups'), ',')::text[])
            AND value::boolean));
      $$,
      i
    );
  END LOOP;
  END
$do$;


DROP POLICY see_extra_perms_user ON usr_to_group;
CREATE POLICY see_extra_perms_user ON public.usr_to_group TO windmill_user USING (true) WITH CHECK ((EXISTS ( SELECT 1
   FROM public.group_
  WHERE (((usr_to_group.group_)::text = (group_.name)::text) AND ((usr_to_group.workspace_id)::text = (group_.workspace_id)::text) AND ((group_.extra_perms ->> concat('u/', current_setting('session.user'::text))))::boolean))));


DROP POLICY see_extra_perms_groups ON usr_to_group;
CREATE POLICY see_extra_perms_groups ON public.usr_to_group TO windmill_user USING (true) WITH CHECK ((EXISTS ( SELECT f.key,
    f.value
   FROM public.group_ g,
    LATERAL jsonb_each_text(g.extra_perms) f(key, value)
  WHERE (((usr_to_group.group_)::text = (g.name)::text) AND ((usr_to_group.workspace_id)::text = (g.workspace_id)::text) AND (split_part(f.key, '/'::text, 1) = 'g'::text) AND (f.key = ANY (regexp_split_to_array(current_setting('session.pgroups'::text), ','::text))) AND (f.value)::boolean))));

