DROP FUNCTION acl_grants_run(JSONB, TEXT[]);

-- Back to booleans, so the casts below hold: a view-only grant becomes a read grant, which runs.
DO $$
DECLARE t TEXT;
BEGIN
    FOREACH t IN ARRAY ARRAY['script', 'flow', 'resource', 'folder'] LOOP
        EXECUTE format($p$UPDATE %1$I SET extra_perms = (
            SELECT jsonb_object_agg(key, value IN ('true', 'write'))
            FROM jsonb_each_text(extra_perms))
            WHERE EXISTS (SELECT 1 FROM jsonb_each(extra_perms) e
                WHERE jsonb_typeof(e.value) = 'string')$p$, t);
    END LOOP;
    FOREACH t IN ARRAY ARRAY['script', 'flow', 'resource'] LOOP
        EXECUTE format($p$ALTER POLICY see_extra_perms_user_insert ON %1$I WITH CHECK (
            ((extra_perms ->> (SELECT concat('u/', current_setting('session.user')))))::boolean)$p$, t);
        EXECUTE format($p$ALTER POLICY see_extra_perms_user_update ON %1$I USING (
            ((extra_perms ->> (SELECT concat('u/', current_setting('session.user')))))::boolean)$p$, t);
        EXECUTE format($p$ALTER POLICY see_extra_perms_user_delete ON %1$I USING (
            ((extra_perms ->> (SELECT concat('u/', current_setting('session.user')))))::boolean)$p$, t);
        EXECUTE format($p$ALTER POLICY see_extra_perms_groups_insert ON %1$I WITH CHECK (
            EXISTS (SELECT 1 FROM jsonb_each_text(%1$I.extra_perms) e
                WHERE split_part(e.key, '/', 1) = 'g'
                    AND e.key = ANY ((SELECT regexp_split_to_array(
                        current_setting('session.pgroups'), ','))::text[])
                    AND e.value::boolean))$p$, t);
        EXECUTE format($p$ALTER POLICY see_extra_perms_groups_update ON %1$I USING (
            EXISTS (SELECT 1 FROM jsonb_each_text(%1$I.extra_perms) e
                WHERE split_part(e.key, '/', 1) = 'g'
                    AND e.key = ANY ((SELECT regexp_split_to_array(
                        current_setting('session.pgroups'), ','))::text[])
                    AND e.value::boolean))$p$, t);
        EXECUTE format($p$ALTER POLICY see_extra_perms_groups_delete ON %1$I USING (
            EXISTS (SELECT 1 FROM jsonb_each_text(%1$I.extra_perms) e
                WHERE split_part(e.key, '/', 1) = 'g'
                    AND e.key = ANY ((SELECT regexp_split_to_array(
                        current_setting('session.pgroups'), ','))::text[])
                    AND e.value::boolean))$p$, t);
    END LOOP;
END $$;
